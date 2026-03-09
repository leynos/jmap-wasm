//! Email-specific JMAP types.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// JMAP method name for email queries.
pub const EMAIL_QUERY_METHOD: &str = "Email/query";

/// JMAP method name for email retrieval.
pub const EMAIL_GET_METHOD: &str = "Email/get";

/// JMAP method name for email updates.
pub const EMAIL_SET_METHOD: &str = "Email/set";

/// Email/query filter.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmailQueryFilter {
    /// Optional mailbox filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_mailbox: Option<String>,
}

/// Email/query arguments.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmailQueryArguments {
    /// Account ID to query.
    pub account_id: String,
    /// Optional filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<EmailQueryFilter>,
    /// Optional offset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<u32>,
    /// Optional limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Whether the server should return a total.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calculate_total: Option<bool>,
}

impl EmailQueryArguments {
    /// Create e-mail query arguments for one account.
    pub fn new(account_id: impl Into<String>) -> Self {
        Self {
            account_id: account_id.into(),
            filter: None,
            position: None,
            limit: None,
            calculate_total: None,
        }
    }

    /// Constrain the query to one mailbox ID.
    #[must_use]
    pub fn in_mailbox(mut self, mailbox_id: impl Into<String>) -> Self {
        self.filter = Some(EmailQueryFilter {
            in_mailbox: Some(mailbox_id.into()),
        });
        self
    }

    /// Set a server-side result limit.
    #[must_use]
    pub const fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Email/query response.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmailQueryResponse {
    /// Account ID that was queried.
    pub account_id: String,
    /// Query state token.
    pub query_state: String,
    /// Whether incremental change calculation is supported.
    pub can_calculate_changes: bool,
    /// Query position returned by the server.
    pub position: u32,
    /// Email IDs returned by the query.
    pub ids: Vec<String>,
    /// Optional total.
    #[serde(default)]
    pub total: Option<u32>,
    /// Optional echoed filter.
    #[serde(default)]
    pub filter: Option<JsonValue>,
    /// Optional echoed sort.
    #[serde(default)]
    pub sort: Option<JsonValue>,
}

/// Email/get arguments.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmailGetArguments {
    /// Account ID to query.
    pub account_id: String,
    /// Email IDs to fetch.
    pub ids: Vec<String>,
    /// Optional property list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<Vec<String>>,
}

impl EmailGetArguments {
    /// Create e-mail get arguments for one account and one or more IDs.
    pub fn new(
        account_id: impl Into<String>,
        ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            account_id: account_id.into(),
            ids: ids.into_iter().map(Into::into).collect(),
            properties: None,
        }
    }

    /// Restrict the response to selected property names.
    #[must_use]
    pub fn with_properties(
        mut self,
        properties: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.properties = Some(properties.into_iter().map(Into::into).collect());
        self
    }
}

/// One e-mail address in JMAP form.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct EmailAddress {
    /// Optional display name.
    #[serde(default)]
    pub name: Option<String>,
    /// Email address.
    pub email: String,
}

/// One text body part reference.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BodyPartReference {
    /// Part ID used as the key in `bodyValues`.
    pub part_id: String,
}

/// One text body value.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BodyValue {
    /// Decoded text value.
    pub value: String,
    /// Whether the server encountered encoding problems.
    #[serde(default)]
    pub is_encoding_problem: Option<bool>,
    /// Whether the value was truncated.
    #[serde(default)]
    pub is_truncated: Option<bool>,
}

/// One JMAP email object.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Email {
    /// Email ID.
    pub id: String,
    /// Optional thread ID.
    #[serde(default)]
    pub thread_id: Option<String>,
    /// Mailbox membership map keyed by mailbox ID.
    #[serde(default)]
    pub mailbox_ids: HashMap<String, bool>,
    /// Keyword map keyed by keyword name.
    #[serde(default)]
    pub keywords: HashMap<String, bool>,
    /// Received timestamp.
    #[serde(default)]
    pub received_at: Option<String>,
    /// Subject.
    #[serde(default)]
    pub subject: Option<String>,
    /// Senders.
    #[serde(default)]
    pub from: Option<Vec<EmailAddress>>,
    /// Recipients.
    #[serde(default)]
    pub to: Option<Vec<EmailAddress>>,
    /// Preview text.
    #[serde(default)]
    pub preview: Option<String>,
    /// Attachment flag.
    #[serde(default)]
    pub has_attachment: Option<bool>,
    /// Text body references.
    #[serde(default)]
    pub text_body: Option<Vec<BodyPartReference>>,
    /// Body values keyed by part ID.
    #[serde(default)]
    pub body_values: Option<HashMap<String, BodyValue>>,
}

impl Email {
    /// Return enabled keyword names in stable lexical order.
    #[must_use]
    pub fn keyword_names(&self) -> Vec<&str> {
        let mut keywords = self
            .keywords
            .iter()
            .filter_map(|(name, enabled)| enabled.then_some(name.as_str()))
            .collect::<Vec<_>>();
        keywords.sort_unstable();
        keywords
    }

    /// Concatenate text body values referenced by `textBody`.
    #[must_use]
    pub fn text_body_value(&self) -> Option<String> {
        let references = self.text_body.as_ref()?;
        let body_values = self.body_values.as_ref()?;
        let body = references
            .iter()
            .filter_map(|reference| body_values.get(&reference.part_id))
            .map(|value| value.value.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        if body.is_empty() { None } else { Some(body) }
    }
}

/// Email/get response.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmailGetResponse {
    /// Account ID that was queried.
    pub account_id: String,
    /// Server state token.
    pub state: String,
    /// Returned emails.
    pub list: Vec<Email>,
    /// IDs that were not found.
    pub not_found: Vec<String>,
}

/// Email/set arguments limited to updates.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EmailSetArguments {
    /// Account ID to update.
    pub account_id: String,
    /// Optional optimistic concurrency token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub if_in_state: Option<String>,
    /// Optional update map.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update: Option<HashMap<String, JsonValue>>,
}

impl EmailSetArguments {
    /// Create a mark-seen update for one e-mail ID.
    pub fn mark_seen(account_id: impl Into<String>, email_id: impl Into<String>) -> Self {
        let mut update = HashMap::new();
        update.insert(email_id.into(), mark_seen_patch());
        Self {
            account_id: account_id.into(),
            if_in_state: None,
            update: Some(update),
        }
    }
}

/// Email/set response.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EmailSetResponse {
    /// Account ID that was updated.
    pub account_id: String,
    /// Previous state token.
    pub old_state: String,
    /// New state token.
    pub new_state: String,
    /// Updated objects, when returned by the server.
    #[serde(default)]
    pub updated: Option<HashMap<String, JsonValue>>,
    /// Update errors keyed by email ID.
    #[serde(default)]
    pub not_updated: Option<HashMap<String, crate::core::JmapError>>,
}

/// Build the JMAP patch that marks one e-mail as seen.
#[must_use]
pub fn mark_seen_patch() -> JsonValue {
    serde_json::json!({ "keywords/$seen": true })
}
