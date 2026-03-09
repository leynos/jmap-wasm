//! End-to-end tests for the native JMAP service path.

use std::{
    collections::HashSet,
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow, bail};
use axum::serve;
use bytes::Bytes;
use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs::Dir};
use reqwest::blocking::Client;
use rusmes_jmap::{JmapServer, Session};
use rusmes_proto::{HeaderMap, Mail, MailAddress, MessageBody, MimeMessage, Username};
use rusmes_storage::{MailboxPath, StorageBackend, backends::filesystem::FilesystemBackend};
use tokio::{net::TcpListener, runtime::Runtime, task::JoinHandle};

use crate::{
    host::{Host, HostHttpRequest, HostHttpResponse, HostLogLevel},
    service::{GetMessageRequest, JmapConfig, JmapService, MarkSeenRequest, NetworkJmapService},
};

const STORAGE_ROOT: &str = "/tmp/rusmes/mail";

#[test]
#[ignore = "requires the rusmes-jmap harness and network access"]
fn native_jmap_service_talks_to_mock_server() -> Result<()> {
    run_rusmes_flow()
}

fn run_rusmes_flow() -> Result<()> {
    reset_storage_root()?;
    let seeded = seed_messages()?;

    let runtime = Runtime::new().context("failed to build Tokio runtime for rusmes-jmap")?;
    let server = runtime
        .block_on(TestServer::start())
        .context("failed to start rusmes-jmap test server")?;
    wait_for_http(&server.base_url, Duration::from_secs(10))?;

    let host = NativeHost::new(server.base_url.clone())?;
    let config = JmapConfig {
        base_url: server.base_url.clone(),
        account_id: None,
        auth_secret_name: Some("jmap_token".to_owned()),
        timeout_ms: 10_000,
    };
    config.verify_secret(&host)?;

    let service = NetworkJmapService;
    let mailboxes = service
        .list_mailboxes(&host, &config)
        .context("listing mailboxes against rusmes-jmap should succeed")?;
    ensure_mailbox_present(&mailboxes.mailboxes, "Inbox")?;

    let message = service
        .get_message(
            &host,
            &GetMessageRequest::new(config.clone(), seeded.message_id.clone()),
        )
        .context("fetching one message against rusmes-jmap should succeed")?;
    if message.message.id != seeded.message_id {
        bail!(
            "rusmes-jmap returned message '{}' instead of seeded '{}'",
            message.message.id,
            seeded.message_id
        );
    }

    let error = service
        .mark_seen(&host, &MarkSeenRequest::new(config, seeded.message_id))
        .expect_err("mark_seen should currently fail against rusmes-jmap");
    if !error.to_string().contains("notImplemented") {
        bail!("mark_seen failure did not mention rusmes-jmap limitation: {error}");
    }

    drop(server);
    drop(runtime);

    Ok(())
}

fn ensure_mailbox_present(
    mailboxes: &[crate::outputs::MailboxInfo],
    expected_name: &str,
) -> Result<()> {
    if mailboxes
        .iter()
        .any(|mailbox| mailbox.name == expected_name)
    {
        return Ok(());
    }

    Err(anyhow!("mailbox list did not contain {expected_name:?}"))
}

fn reset_storage_root() -> Result<()> {
    let root_dir = Dir::open_ambient_dir("/", ambient_authority())
        .context("failed to open ambient root for rusmes-jmap cleanup")?;
    let storage_path = Utf8Path::new(STORAGE_ROOT);
    let relative_path = storage_path
        .strip_prefix("/")
        .with_context(|| format!("storage root '{storage_path}' must be an absolute path"))?;

    match root_dir.remove_dir_all(relative_path.as_std_path()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("failed to clear '{storage_path}'")),
    }
}

fn seed_messages() -> Result<SeededMessage> {
    let storage_root = Utf8PathBuf::from(STORAGE_ROOT);
    let runtime = Runtime::new().context("failed to build Tokio runtime for seeding")?;
    runtime.block_on(async {
        let backend = FilesystemBackend::new(storage_root.as_std_path().to_path_buf());
        let mailbox_store = backend.mailbox_store();
        let message_store = backend.message_store();

        let username: Username = "user@example.com"
            .parse()
            .context("failed to parse seeded username")?;
        let path = MailboxPath::new(username, vec!["INBOX".to_owned()]);
        let mailbox_id = mailbox_store
            .create_mailbox(&path)
            .await
            .context("failed to create seeded mailbox")?;

        let mail = Mail::new(
            Some(
                "sender@example.com"
                    .parse::<MailAddress>()
                    .context("failed to parse sender address")?,
            ),
            vec![
                "user@example.com"
                    .parse::<MailAddress>()
                    .context("failed to parse recipient address")?,
            ],
            MimeMessage::new(
                HeaderMap::new(),
                MessageBody::Small(Bytes::from(
                    "Subject: Seeded message\r\n\r\nHello from rusmes-jmap",
                )),
            ),
            None,
            None,
        );

        let metadata = message_store
            .append_message(&mailbox_id, mail)
            .await
            .context("failed to append seeded message")?;
        Ok::<SeededMessage, anyhow::Error>(SeededMessage {
            message_id: metadata.message_id().to_string(),
        })
    })
}

struct SeededMessage {
    message_id: String,
}

struct TestServer {
    base_url: String,
    task: JoinHandle<()>,
}

impl TestServer {
    async fn start() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind rusmes-jmap listener")?;
        let address = listener
            .local_addr()
            .context("failed to read rusmes-jmap listener address")?;
        let task = tokio::spawn(async move {
            if let Err(error) = serve(listener, JmapServer::routes()).await {
                panic!("rusmes-jmap server failed: {error}");
            }
        });

        Ok(Self {
            base_url: format!("http://{address}"),
            task,
        })
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

struct NativeHost {
    base_url: String,
    client: Client,
    secrets: HashSet<String>,
}

impl NativeHost {
    fn new(base_url: String) -> Result<Self> {
        Ok(Self {
            base_url,
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .context("failed to build reqwest client for native host")?,
            secrets: HashSet::from(["jmap_token".to_owned()]),
        })
    }
}

impl Host for NativeHost {
    fn log(&self, _level: HostLogLevel, _message: &str) {}

    fn secret_exists(&self, name: &str) -> bool {
        self.secrets.contains(name)
    }

    fn http_request(
        &self,
        request: &HostHttpRequest,
    ) -> Result<HostHttpResponse, crate::errors::ToolError> {
        if request.method == "GET" && request.url.ends_with("/.well-known/jmap") {
            let session = Session::new(
                "user@example.com".to_owned(),
                "acc-1".to_owned(),
                self.base_url.clone(),
            );
            let body =
                serde_json::to_vec(&session).map_err(crate::errors::ToolError::SerializeOutput)?;
            return Ok(HostHttpResponse {
                status: 200,
                headers: serde_json::Map::new(),
                body,
            });
        }

        let method = reqwest::Method::from_bytes(request.method.as_bytes())
            .map_err(|error| crate::errors::ToolError::HostHttp(error.to_string()))?;
        let mut builder = self.client.request(method, &request.url);
        let header_values = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(
            &request.headers_json,
        )
        .map_err(crate::errors::ToolError::InvalidHeadersJson)?;
        for (name, header_value) in header_values {
            if let Some(header_text) = header_value.as_str() {
                builder = builder.header(&name, header_text);
            }
        }
        if let Some(body) = &request.body {
            builder = builder.body(body.clone());
        }
        let response = builder
            .send()
            .map_err(|error| crate::errors::ToolError::HostHttp(error.to_string()))?;
        let status = response.status().as_u16();
        let mut headers = serde_json::Map::new();
        for (name, header_value) in response.headers() {
            if let Ok(header_text) = header_value.to_str() {
                let _ = headers.insert(
                    name.as_str().to_owned(),
                    serde_json::Value::String(header_text.to_owned()),
                );
            }
        }
        let body = response
            .bytes()
            .map_err(|error| crate::errors::ToolError::HostHttp(error.to_string()))?
            .to_vec();

        Ok(HostHttpResponse {
            status,
            headers,
            body,
        })
    }
}

fn wait_for_http(base_url: &str, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    let client = Client::builder()
        .timeout(Duration::from_secs(1))
        .build()
        .context("failed to build HTTP wait client")?;

    loop {
        match client.get(format!("{base_url}/.well-known/jmap")).send() {
            Ok(_) => return Ok(()),
            Err(error) if Instant::now() < deadline => {
                let _ = error;
                thread::sleep(Duration::from_millis(200));
            }
            Err(error) => {
                return Err(error).context("timed out waiting for rusmes-jmap HTTP endpoint");
            }
        }
    }
}
