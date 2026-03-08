//! Docker-backed end-to-end tests for the native IMAP service path.

use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow, bail};

use crate::{
    host::{Host, HostLogLevel},
    service::{
        ConnectionConfig, GetMessageRequest, ImapService, ListMessagesRequest, MarkSeenRequest,
        NetworkImapService,
    },
};

const GREENMAIL_IMAGE: &str = "docker.io/greenmail/standalone";
const IMAP_HOST: &str = "127.0.0.1";
const IMAP_PORT: u16 = 3143;
const SMTP_PORT: u16 = 3025;
const ACCOUNT: &str = "alice@localhost";

#[test]
#[ignore = "requires docker and network access"]
fn network_imap_service_talks_to_greenmail() {
    run_greenmail_flow().expect("GreenMail e2e should pass");
}

fn run_greenmail_flow() -> Result<()> {
    let _container = GreenMailContainer::start()?;
    seed_messages()?;

    let connection = ConnectionConfig {
        host: IMAP_HOST.to_owned(),
        port: IMAP_PORT,
        username: ACCOUNT.to_owned(),
        password: ACCOUNT.to_owned(),
        password_secret_name: Some("imap_password".to_owned()),
    };
    connection
        .verify_secret(&SecretHost)
        .context("secret preflight should pass in GreenMail e2e")?;

    let service = NetworkImapService;
    let mailboxes = wait_for_mailboxes(&service, &connection, Duration::from_secs(10))?;
    ensure_mailbox_present(&mailboxes.mailboxes, "INBOX")?;

    let messages = wait_for_messages(&service, &connection, 2, Duration::from_secs(10))?;
    ensure_subject_present(&messages.messages, "First message")?;
    ensure_subject_present(&messages.messages, "Second message")?;

    let message = service
        .get_message(&GetMessageRequest::new(
            connection.clone(),
            Some("INBOX".to_owned()),
            1,
        ))
        .context("fetching one message against GreenMail should succeed")?;
    ensure_contains(
        message.message.body.as_deref(),
        "GreenMail delivery one",
        "message body",
    )?;

    let updated = service
        .mark_seen(&MarkSeenRequest::new(
            connection,
            Some("INBOX".to_owned()),
            1,
        ))
        .context("marking a message as seen against GreenMail should succeed")?;
    if !updated.seen {
        bail!("mark_seen returned seen=false for the first message");
    }

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

fn ensure_subject_present(
    messages: &[crate::outputs::MessageSummary],
    expected_subject: &str,
) -> Result<()> {
    if messages
        .iter()
        .any(|message| message.subject.as_deref() == Some(expected_subject))
    {
        return Ok(());
    }

    Err(anyhow!(
        "message list did not contain subject {expected_subject:?}"
    ))
}

fn ensure_contains(
    maybe_value: Option<&str>,
    expected_fragment: &str,
    context: &str,
) -> Result<()> {
    let Some(actual_value) = maybe_value else {
        bail!("{context} was missing");
    };
    if actual_value.contains(expected_fragment) {
        return Ok(());
    }

    Err(anyhow!(
        "{context} did not contain {expected_fragment:?}: {actual_value}"
    ))
}

fn wait_for_mailboxes(
    service: &NetworkImapService,
    connection: &ConnectionConfig,
    timeout: Duration,
) -> Result<crate::outputs::ListMailboxesOutput> {
    let deadline = Instant::now() + timeout;

    loop {
        match service.list_mailboxes(connection) {
            Ok(mailboxes) => return Ok(mailboxes),
            Err(error) if Instant::now() < deadline => {
                let _ = error;
                thread::sleep(Duration::from_millis(200));
            }
            Err(error) => {
                return Err(error).context("listing mailboxes against GreenMail should succeed");
            }
        }
    }
}

fn wait_for_messages(
    service: &NetworkImapService,
    connection: &ConnectionConfig,
    expected_count: usize,
    timeout: Duration,
) -> Result<crate::outputs::ListMessagesOutput> {
    let deadline = Instant::now() + timeout;

    loop {
        let messages = service
            .list_messages(&ListMessagesRequest::new(
                connection.clone(),
                Some("INBOX".to_owned()),
                Some("1:*".to_owned()),
            ))
            .context("listing messages against GreenMail should succeed")?;
        if messages.messages.len() >= expected_count {
            return Ok(messages);
        }
        if Instant::now() >= deadline {
            bail!(
                "timed out waiting for {expected_count} IMAP messages, saw {}",
                messages.messages.len()
            );
        }

        thread::sleep(Duration::from_millis(200));
    }
}

struct GreenMailContainer {
    name: String,
}

impl GreenMailContainer {
    fn start() -> Result<Self> {
        let name = format!("imap-wasm-greenmail-{}", std::process::id());
        let options = concat!(
            "-Dgreenmail.setup.test.smtp ",
            "-Dgreenmail.setup.test.imap ",
            "-Dgreenmail.hostname=0.0.0.0"
        );

        let status = Command::new("docker")
            .args([
                "run",
                "--detach",
                "--rm",
                "--name",
                &name,
                "-p",
                "127.0.0.1:3025:3025",
                "-p",
                "127.0.0.1:3143:3143",
                "-e",
                &format!("GREENMAIL_OPTS={options}"),
                GREENMAIL_IMAGE,
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context("failed to start GreenMail container")?;
        if !status.success() {
            bail!("docker run returned a non-zero exit status for GreenMail");
        }

        wait_for_port(&format!("{IMAP_HOST}:{IMAP_PORT}"), Duration::from_secs(30))?;
        wait_for_port(&format!("{IMAP_HOST}:{SMTP_PORT}"), Duration::from_secs(30))?;
        wait_for_smtp_banner(&format!("{IMAP_HOST}:{SMTP_PORT}"), Duration::from_secs(30))?;

        Ok(Self { name })
    }
}

impl Drop for GreenMailContainer {
    fn drop(&mut self) {
        if let Err(_error) = Command::new("docker")
            .args(["rm", "--force", &self.name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {}
    }
}

struct SecretHost;

impl Host for SecretHost {
    fn log(&self, _level: HostLogLevel, _message: &str) {}

    fn secret_exists(&self, name: &str) -> bool {
        name == "imap_password"
    }
}

fn wait_for_port(address: &str, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;

    loop {
        match TcpStream::connect(address) {
            Ok(stream) => {
                drop(stream);
                return Ok(());
            }
            Err(error) if Instant::now() < deadline => {
                let _ = error;
                thread::sleep(Duration::from_millis(200));
            }
            Err(error) => {
                return Err(error).with_context(|| format!("timed out waiting for {address}"));
            }
        }
    }
}

fn wait_for_smtp_banner(address: &str, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;

    loop {
        match TcpStream::connect(address) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .context("failed to set SMTP banner timeout")?;
                let mut reader = BufReader::new(stream);
                let smtp_status = read_smtp_response(&mut reader);
                if matches!(smtp_status, Ok((220, _))) {
                    return Ok(());
                }
                if Instant::now() < deadline {
                    thread::sleep(Duration::from_millis(200));
                    continue;
                }

                return smtp_banner_timeout(address, smtp_status);
            }
            Err(error) if Instant::now() < deadline => {
                let _ = error;
                thread::sleep(Duration::from_millis(200));
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("timed out connecting for SMTP greeting at {address}")
                });
            }
        }
    }
}

fn smtp_banner_timeout(address: &str, smtp_status: Result<(u16, String)>) -> Result<()> {
    match smtp_status {
        Ok(_) => bail!("timed out waiting for SMTP greeting at {address}"),
        Err(error) => {
            Err(error).with_context(|| format!("timed out waiting for SMTP greeting at {address}"))
        }
    }
}

fn seed_messages() -> Result<()> {
    let smtp_address = format!("{IMAP_HOST}:{SMTP_PORT}");
    send_message(&smtp_address, "First message", "GreenMail delivery one")?;
    send_message(&smtp_address, "Second message", "GreenMail delivery two")
}

fn send_message(address: &str, subject: &str, body: &str) -> Result<()> {
    let mut stream = TcpStream::connect(address)
        .with_context(|| format!("failed to connect to SMTP server at {address}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .context("failed to set SMTP read timeout")?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .context("failed to set SMTP write timeout")?;

    let mut reader = BufReader::new(stream.try_clone().context("failed to clone SMTP stream")?);
    expect_code(&mut reader, 220)?;
    send_command(&mut stream, &mut reader, "HELO localhost\r\n", 250)?;
    send_command(
        &mut stream,
        &mut reader,
        "MAIL FROM:<sender@example.com>\r\n",
        250,
    )?;
    send_command(
        &mut stream,
        &mut reader,
        &format!("RCPT TO:<{ACCOUNT}>\r\n"),
        250,
    )?;
    send_command(&mut stream, &mut reader, "DATA\r\n", 354)?;
    stream
        .write_all(
            format!(
                "From: sender@example.com\r\nTo: {ACCOUNT}\r\nSubject: {subject}\r\n\r\n{body}\r\n.\r\n"
            )
            .as_bytes(),
        )
        .context("failed to send SMTP message body")?;
    stream
        .flush()
        .context("failed to flush SMTP message body")?;
    expect_code(&mut reader, 250)?;
    send_command(&mut stream, &mut reader, "QUIT\r\n", 221)?;

    Ok(())
}

fn send_command(
    stream: &mut TcpStream,
    reader: &mut BufReader<TcpStream>,
    command: &str,
    expected_code: u16,
) -> Result<()> {
    stream
        .write_all(command.as_bytes())
        .with_context(|| format!("failed to send SMTP command {command:?}"))?;
    stream.flush().context("failed to flush SMTP command")?;
    expect_code(reader, expected_code)
}

fn expect_code(reader: &mut BufReader<TcpStream>, expected_code: u16) -> Result<()> {
    let (code, line) = read_smtp_response(reader)?;
    if code != expected_code {
        bail!(
            "expected SMTP {expected_code}, got {code}: {}",
            line.trim_end()
        );
    }

    Ok(())
}

fn read_smtp_response(reader: &mut BufReader<TcpStream>) -> Result<(u16, String)> {
    let mut line = String::new();
    let code = loop {
        line.clear();
        reader
            .read_line(&mut line)
            .context("failed to read SMTP response")?;
        let parsed_code = line
            .get(0..3)
            .ok_or_else(|| anyhow!("missing SMTP status code in response {line:?}"))?
            .parse::<u16>()
            .context("failed to parse SMTP status code")?;
        let Some(separator) = line.get(3..4) else {
            break parsed_code;
        };
        if separator == " " {
            break parsed_code;
        }
    };

    Ok((code, line))
}
