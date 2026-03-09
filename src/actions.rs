//! Request parsing and action dispatch for the JMAP tool.

use serde::Deserialize;

use crate::{
    errors::ToolError,
    host::Host,
    outputs::ActionOutput,
    service::{GetMessageRequest, JmapConfig, JmapService, ListMessagesRequest, MarkSeenRequest},
};

/// Parsed JMAP tool request.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub(crate) enum JmapAction {
    /// Enumerate available mailboxes.
    ListMailboxes {
        #[serde(flatten)]
        config: JmapConfig,
    },
    /// List message summaries from one mailbox or from the account as a whole.
    ListMessages {
        #[serde(flatten)]
        config: JmapConfig,
        mailbox_id: Option<String>,
        mailbox_name: Option<String>,
        limit: Option<u32>,
        position: Option<u32>,
    },
    /// Fetch one message including its text body.
    GetMessage {
        #[serde(flatten)]
        config: JmapConfig,
        email_id: String,
    },
    /// Add the `$seen` keyword to one message.
    MarkSeen {
        #[serde(flatten)]
        config: JmapConfig,
        email_id: String,
    },
}

impl JmapAction {
    /// Parse JSON parameters into an action.
    pub(crate) fn parse(params: &str) -> Result<Self, ToolError> {
        serde_json::from_str(params).map_err(ToolError::InvalidParameters)
    }

    /// Return the external action identifier.
    pub(crate) const fn action_name(&self) -> &'static str {
        match self {
            Self::ListMailboxes { .. } => "list_mailboxes",
            Self::ListMessages { .. } => "list_messages",
            Self::GetMessage { .. } => "get_message",
            Self::MarkSeen { .. } => "mark_seen",
        }
    }

    /// Check any requested secret-existence preflight.
    pub(crate) fn verify_secret<H: Host>(&self, host: &H) -> Result<(), ToolError> {
        let config = match self {
            Self::ListMailboxes { config }
            | Self::ListMessages { config, .. }
            | Self::GetMessage { config, .. }
            | Self::MarkSeen { config, .. } => config,
        };

        config.verify_secret(host)
    }

    /// Execute one parsed action against the selected service implementation.
    pub(crate) fn execute<H: Host, S: JmapService>(
        &self,
        host: &H,
        service: &S,
    ) -> Result<ActionOutput, ToolError> {
        match self {
            Self::ListMailboxes { config } => service
                .list_mailboxes(host, config)
                .map(ActionOutput::ListMailboxes),
            Self::ListMessages {
                config,
                mailbox_id,
                mailbox_name,
                limit,
                position,
            } => {
                let request = ListMessagesRequest {
                    config: config.clone(),
                    mailbox_id: mailbox_id.clone(),
                    mailbox_name: mailbox_name.clone(),
                    limit: limit.unwrap_or(20),
                    position: position.unwrap_or_default(),
                };
                request.validate()?;
                service
                    .list_messages(host, &request)
                    .map(ActionOutput::ListMessages)
            }
            Self::GetMessage { config, email_id } => service
                .get_message(
                    host,
                    &GetMessageRequest::new(config.clone(), email_id.clone()),
                )
                .map(ActionOutput::GetMessage),
            Self::MarkSeen { config, email_id } => service
                .mark_seen(
                    host,
                    &MarkSeenRequest::new(config.clone(), email_id.clone()),
                )
                .map(ActionOutput::MarkSeen),
        }
    }
}
