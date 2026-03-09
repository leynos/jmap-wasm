//! Mailbox-specific JMAP types.

use serde::{Deserialize, Serialize};

/// JMAP method name for mailbox retrieval.
pub const MAILBOX_GET_METHOD: &str = "Mailbox/get";

/// Mailbox/get arguments.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MailboxGetArguments {
    /// Account ID to query.
    pub account_id: String,
    /// Optional mailbox IDs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ids: Option<Vec<String>>,
    /// Optional mailbox properties.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<Vec<String>>,
}

impl MailboxGetArguments {
    /// Create mailbox-get arguments for one account.
    pub fn new(account_id: impl Into<String>) -> Self {
        Self {
            account_id: account_id.into(),
            ids: None,
            properties: None,
        }
    }

    /// Restrict the request to selected mailbox IDs.
    #[must_use]
    pub fn with_ids(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.ids = Some(ids.into_iter().map(Into::into).collect());
        self
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

/// One JMAP mailbox object.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Mailbox {
    /// Mailbox ID.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Optional role.
    #[serde(default)]
    pub role: Option<String>,
    /// Optional parent mailbox ID.
    #[serde(default)]
    pub parent_id: Option<String>,
    /// Optional sort order.
    #[serde(default)]
    pub sort_order: Option<u32>,
    /// Optional subscription flag.
    #[serde(default)]
    pub is_subscribed: Option<bool>,
    /// Optional total email count.
    #[serde(default)]
    pub total_emails: Option<u64>,
    /// Optional unread email count.
    #[serde(default)]
    pub unread_emails: Option<u64>,
}

/// Mailbox/get response.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MailboxGetResponse {
    /// Account ID that was queried.
    pub account_id: String,
    /// Server state token.
    pub state: String,
    /// Returned mailboxes.
    pub list: Vec<Mailbox>,
    /// IDs that were not found.
    pub not_found: Vec<String>,
}

impl MailboxGetResponse {
    /// Find the first mailbox whose name matches `mailbox_name`.
    #[must_use]
    pub fn find_by_name(&self, mailbox_name: &str) -> Option<&Mailbox> {
        self.list
            .iter()
            .find(|mailbox| mailbox.name == mailbox_name)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::{Mailbox, MailboxGetArguments, MailboxGetResponse};

    #[rstest]
    fn mailbox_get_arguments_capture_ids_and_properties() {
        let args = MailboxGetArguments::new("account-1")
            .with_ids(["inbox"])
            .with_properties(["id", "name"]);

        assert_eq!(args.ids, Some(vec!["inbox".to_owned()]));
        assert_eq!(
            args.properties,
            Some(vec!["id".to_owned(), "name".to_owned()])
        );
    }

    #[rstest]
    fn mailbox_lookup_finds_by_name() {
        let response = MailboxGetResponse {
            account_id: "account-1".to_owned(),
            state: "state-1".to_owned(),
            list: vec![Mailbox {
                id: "inbox".to_owned(),
                name: "Inbox".to_owned(),
                role: None,
                parent_id: None,
                sort_order: None,
                is_subscribed: None,
                total_emails: None,
                unread_emails: None,
            }],
            not_found: Vec::new(),
        };

        assert_eq!(
            response
                .find_by_name("Inbox")
                .map(|mailbox| mailbox.id.as_str()),
            Some("inbox")
        );
    }
}
