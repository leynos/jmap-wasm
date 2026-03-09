//! Service abstractions for JMAP-backed mail actions.

use serde::Deserialize;

use jmap_codec::{
    core::MAIL_CAPABILITY,
    mail::{
        EmailGetArguments, EmailQueryArguments, EmailQueryFilter, EmailSetArguments,
        MailboxGetArguments,
    },
};
use serde_json::json;

use crate::{
    errors::ToolError,
    host::Host,
    jmap_transport,
    mappers::{
        map_mailbox, map_message_detail, map_message_summary, message_properties,
        message_properties_with_body, truthy_map_keys,
    },
    outputs::{GetMessageOutput, ListMailboxesOutput, ListMessagesOutput, MarkSeenOutput},
};

/// Shared JMAP configuration.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct JmapConfig {
    /// Base URL of the JMAP server.
    pub(crate) base_url: String,
    /// Optional account ID override.
    pub(crate) account_id: Option<String>,
    /// Optional auth secret name used for `secret_exists` preflight.
    pub(crate) auth_secret_name: Option<String>,
    /// Per-request timeout for the host HTTP bridge.
    #[serde(default = "default_timeout_ms")]
    pub(crate) timeout_ms: u32,
}

impl JmapConfig {
    /// Verify that any optional secret preflight passes.
    pub(crate) fn verify_secret<H: Host>(&self, host: &H) -> Result<(), ToolError> {
        if let Some(secret_name) = &self.auth_secret_name
            && !host.secret_exists(secret_name)
        {
            return Err(ToolError::MissingSecret(secret_name.clone()));
        }
        Ok(())
    }
}

/// Request for listing messages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ListMessagesRequest {
    /// Connection details.
    pub(crate) config: JmapConfig,
    /// Optional mailbox ID filter.
    pub(crate) mailbox_id: Option<String>,
    /// Optional mailbox name filter.
    pub(crate) mailbox_name: Option<String>,
    /// Maximum number of messages to return.
    pub(crate) limit: u32,
    /// Query start position.
    pub(crate) position: u32,
}

impl ListMessagesRequest {
    /// Validate that the mailbox selectors are coherent.
    pub(crate) fn validate(&self) -> Result<(), ToolError> {
        if self.mailbox_id.is_some() && self.mailbox_name.is_some() {
            return Err(ToolError::InvalidRequest(
                "Provide either mailbox_id or mailbox_name, not both".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Request for fetching one message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GetMessageRequest {
    /// Connection details.
    pub(crate) config: JmapConfig,
    /// Message ID.
    pub(crate) email_id: String,
}

impl GetMessageRequest {
    /// Construct a fetch request.
    pub(crate) const fn new(config: JmapConfig, email_id: String) -> Self {
        Self { config, email_id }
    }
}

/// Request for updating one message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MarkSeenRequest {
    /// Connection details.
    pub(crate) config: JmapConfig,
    /// Message ID.
    pub(crate) email_id: String,
}

impl MarkSeenRequest {
    /// Construct a flag-update request.
    pub(crate) const fn new(config: JmapConfig, email_id: String) -> Self {
        Self { config, email_id }
    }
}

/// Transport-agnostic JMAP operations used by request execution.
pub(crate) trait JmapService {
    /// List the mailboxes visible to the logged-in account.
    fn list_mailboxes<H: Host>(
        &self,
        host: &H,
        config: &JmapConfig,
    ) -> Result<ListMailboxesOutput, ToolError>;

    /// List messages from one mailbox.
    fn list_messages<H: Host>(
        &self,
        host: &H,
        request: &ListMessagesRequest,
    ) -> Result<ListMessagesOutput, ToolError>;

    /// Fetch one full message.
    fn get_message<H: Host>(
        &self,
        host: &H,
        request: &GetMessageRequest,
    ) -> Result<GetMessageOutput, ToolError>;

    /// Mark one message as seen.
    fn mark_seen<H: Host>(
        &self,
        host: &H,
        request: &MarkSeenRequest,
    ) -> Result<MarkSeenOutput, ToolError>;
}

/// Production JMAP implementation backed by the Ironclaw host HTTP bridge.
pub(crate) struct NetworkJmapService;

impl JmapService for NetworkJmapService {
    fn list_mailboxes<H: Host>(
        &self,
        host: &H,
        config: &JmapConfig,
    ) -> Result<ListMailboxesOutput, ToolError> {
        let session = jmap_transport::discover_session(host, config)?;
        let account_id = resolve_account_id(config, &session)?;
        let response = jmap_transport::mailbox_get(
            host,
            config,
            &session.api_url,
            MailboxGetArguments {
                account_id: account_id.clone(),
                ids: None,
                properties: None,
            },
        )?;

        Ok(ListMailboxesOutput {
            account_id,
            mailboxes: response.list.into_iter().map(map_mailbox).collect(),
        })
    }

    fn list_messages<H: Host>(
        &self,
        host: &H,
        request: &ListMessagesRequest,
    ) -> Result<ListMessagesOutput, ToolError> {
        let session = jmap_transport::discover_session(host, &request.config)?;
        let account_id = resolve_account_id(&request.config, &session)?;
        let context = ServiceContext {
            config: &request.config,
            api_url: &session.api_url,
            account_id: &account_id,
        };
        let mailbox_id = resolve_mailbox_id(host, &context, request)?;

        let query = jmap_transport::email_query(
            host,
            context.config,
            context.api_url,
            EmailQueryArguments {
                account_id: account_id.clone(),
                filter: mailbox_id.clone().map(|id| EmailQueryFilter {
                    in_mailbox: Some(id),
                }),
                position: Some(request.position),
                limit: Some(request.limit),
                calculate_total: Some(true),
            },
        )?;

        let messages = if query.ids.is_empty() {
            Vec::new()
        } else {
            jmap_transport::email_get(
                host,
                &request.config,
                &session.api_url,
                EmailGetArguments {
                    account_id: account_id.clone(),
                    ids: query.ids,
                    properties: Some(message_properties()),
                },
            )?
            .list
            .into_iter()
            .map(map_message_summary)
            .collect()
        };

        Ok(ListMessagesOutput {
            account_id,
            mailbox_id,
            position: query.position,
            total: query.total,
            messages,
        })
    }

    fn get_message<H: Host>(
        &self,
        host: &H,
        request: &GetMessageRequest,
    ) -> Result<GetMessageOutput, ToolError> {
        let session = jmap_transport::discover_session(host, &request.config)?;
        let account_id = resolve_account_id(&request.config, &session)?;
        let response = jmap_transport::email_get(
            host,
            &request.config,
            &session.api_url,
            EmailGetArguments {
                account_id: account_id.clone(),
                ids: vec![request.email_id.clone()],
                properties: Some(message_properties_with_body()),
            },
        )?;
        let message = response.list.into_iter().next().ok_or_else(|| {
            ToolError::InvalidRequest(format!("Email '{}' was not found", request.email_id))
        })?;

        Ok(GetMessageOutput {
            account_id,
            message: map_message_detail(message),
        })
    }

    fn mark_seen<H: Host>(
        &self,
        host: &H,
        request: &MarkSeenRequest,
    ) -> Result<MarkSeenOutput, ToolError> {
        let session = jmap_transport::discover_session(host, &request.config)?;
        let account_id = resolve_account_id(&request.config, &session)?;
        let current = jmap_transport::email_get(
            host,
            &request.config,
            &session.api_url,
            EmailGetArguments {
                account_id: account_id.clone(),
                ids: vec![request.email_id.clone()],
                properties: Some(vec!["keywords".to_owned()]),
            },
        )?
        .list
        .into_iter()
        .next()
        .ok_or_else(|| {
            ToolError::InvalidRequest(format!("Email '{}' was not found", request.email_id))
        })?;
        let mut keywords = current.keywords;
        let _ = keywords.insert("$seen".to_owned(), true);

        let mut updates = std::collections::HashMap::new();
        let _ = updates.insert(
            request.email_id.clone(),
            json!({
                "keywords": keywords,
            }),
        );

        let response = jmap_transport::email_set(
            host,
            &request.config,
            &session.api_url,
            EmailSetArguments {
                account_id: account_id.clone(),
                if_in_state: None,
                update: Some(updates),
            },
        )?;
        if let Some(not_updated) = response.not_updated
            && let Some(error) = not_updated.get(&request.email_id)
        {
            return Err(ToolError::InvalidResponse(format!(
                "Email/set rejected '{}': {}{}",
                request.email_id,
                error.error_type,
                if error.description.is_empty() {
                    String::new()
                } else {
                    format!(": {}", error.description)
                }
            )));
        }

        let keyword_list = truthy_map_keys(&keywords);
        Ok(MarkSeenOutput {
            account_id,
            email_id: request.email_id.clone(),
            seen: keyword_list.iter().any(|keyword| keyword == "$seen"),
            keywords: keyword_list,
        })
    }
}

fn resolve_account_id(
    config: &JmapConfig,
    session: &jmap_codec::SessionResource,
) -> Result<String, ToolError> {
    config.account_id.clone().map_or_else(
        || {
            session
                .primary_account(MAIL_CAPABILITY)
                .map(str::to_owned)
                .ok_or_else(|| {
                    ToolError::InvalidResponse(
                        "Session resource did not advertise a primary mail account".to_owned(),
                    )
                })
        },
        Ok,
    )
}

struct ServiceContext<'a> {
    config: &'a JmapConfig,
    api_url: &'a str,
    account_id: &'a str,
}

fn resolve_mailbox_id<H: Host>(
    host: &H,
    context: &ServiceContext<'_>,
    request: &ListMessagesRequest,
) -> Result<Option<String>, ToolError> {
    if let Some(existing_mailbox_id) = request.mailbox_id.as_deref() {
        return Ok(Some(existing_mailbox_id.to_owned()));
    }
    let Some(requested_mailbox_name) = request.mailbox_name.as_deref() else {
        return Ok(None);
    };

    let response = jmap_transport::mailbox_get(
        host,
        context.config,
        context.api_url,
        MailboxGetArguments {
            account_id: context.account_id.to_owned(),
            ids: None,
            properties: None,
        },
    )?;

    response
        .list
        .into_iter()
        .find(|mailbox| mailbox.name == requested_mailbox_name)
        .map(|mailbox| Some(mailbox.id))
        .ok_or_else(|| {
            ToolError::InvalidRequest(format!("Mailbox '{requested_mailbox_name}' was not found"))
        })
}
const fn default_timeout_ms() -> u32 {
    30_000
}
