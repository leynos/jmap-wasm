//! Core JMAP session, request, and response types.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// JMAP core capability URN.
pub const CORE_CAPABILITY: &str = "urn:ietf:params:jmap:core";

/// JMAP mail capability URN.
pub const MAIL_CAPABILITY: &str = "urn:ietf:params:jmap:mail";

/// Opaque capability data keyed by JMAP capability URN.
pub type CapabilityMap = HashMap<String, Value>;

/// Backwards-compatible alias for session capability maps.
pub type SessionCapabilities = CapabilityMap;

/// JMAP session resource returned from `/.well-known/jmap`.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResource {
    /// Server capabilities keyed by capability URN.
    pub capabilities: CapabilityMap,
    /// Accounts visible to the authenticated principal.
    pub accounts: HashMap<String, AccountResource>,
    /// Primary account IDs keyed by capability URN.
    pub primary_accounts: HashMap<String, String>,
    /// Username associated with the session.
    pub username: String,
    /// API URL for POST JMAP method calls.
    pub api_url: String,
    /// Blob download URL template.
    pub download_url: String,
    /// Blob upload URL template.
    pub upload_url: String,
    /// Optional event source URL.
    pub event_source_url: Option<String>,
    /// Opaque session state token.
    pub state: String,
}

impl SessionResource {
    /// Return the primary account ID for one capability, if present.
    #[must_use]
    pub fn primary_account(&self, capability: &str) -> Option<&str> {
        self.primary_accounts.get(capability).map(String::as_str)
    }
}

/// Minimal per-account session data.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountResource {
    /// Human-readable account name.
    pub name: Option<String>,
    /// Whether the account is personal to the user.
    pub is_personal: Option<bool>,
    /// Whether the account is read-only.
    pub is_read_only: Option<bool>,
    /// Account-scoped capability data.
    pub account_capabilities: Option<CapabilityMap>,
}

/// Top-level JMAP request object.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestObject {
    /// Capabilities declared for the request.
    pub using: Vec<String>,
    /// Method invocations to execute.
    #[serde(rename = "methodCalls")]
    pub method_calls: Vec<Invocation>,
    /// Optional client-side creation ID mapping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_ids: Option<HashMap<String, String>>,
}

impl RequestObject {
    /// Create a new request with the provided capabilities.
    #[must_use]
    pub fn new(capabilities: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            using: capabilities.into_iter().map(Into::into).collect(),
            method_calls: Vec::new(),
            created_ids: None,
        }
    }

    /// Create a request declaring the standard core and mail capabilities.
    #[must_use]
    pub fn mail() -> Self {
        Self::new([CORE_CAPABILITY, MAIL_CAPABILITY])
    }

    /// Append one serialized method call.
    ///
    /// # Errors
    ///
    /// Returns an error when `arguments` cannot be serialized into JSON.
    pub fn with_method_call<T: Serialize>(
        mut self,
        method_name: &str,
        arguments: &T,
        call_id: &str,
    ) -> Result<Self, serde_json::Error> {
        self.method_calls.push(Invocation(
            method_name.to_owned(),
            serde_json::to_value(arguments)?,
            call_id.to_owned(),
        ));
        Ok(self)
    }
}

/// One JMAP method invocation encoded as `[name, args, callId]`.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Invocation(pub String, pub Value, pub String);

/// Envelope returned by the JMAP API.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JmapResponse<T> {
    /// Method responses returned by the server.
    #[serde(rename = "methodResponses")]
    pub method_responses: Vec<MethodResponse<T>>,
    /// Opaque session state token after the call.
    pub session_state: String,
    /// Optional creation ID mapping returned by the server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_ids: Option<HashMap<String, String>>,
}

impl<T> JmapResponse<T> {
    /// Extract the only method response and validate its metadata.
    ///
    /// # Errors
    ///
    /// Returns an error when the response contains zero or multiple method
    /// responses, or when the single response metadata does not match the
    /// expected values.
    pub fn into_single_response(
        mut self,
        expected_method: &str,
        expected_call_id: &str,
    ) -> Result<MethodResponse<T>, ResponseShapeError> {
        let response = match self.method_responses.len() {
            0 => return Err(ResponseShapeError::NoMethodResponses),
            1 => self
                .method_responses
                .pop()
                .ok_or(ResponseShapeError::NoMethodResponses)?,
            count => return Err(ResponseShapeError::UnexpectedMethodCount { count }),
        };
        response.expect_metadata(expected_method, expected_call_id)
    }
}

/// One JMAP method response tuple.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MethodResponse<T>(pub String, pub MethodResponseData<T>, pub String);

impl<T> MethodResponse<T> {
    /// Ensure the method metadata matches one expected response.
    ///
    /// # Errors
    ///
    /// Returns an error when the method name or call ID does not match the
    /// expected values.
    pub fn expect_metadata(
        self,
        expected_method: &str,
        expected_call_id: &str,
    ) -> Result<Self, ResponseShapeError> {
        if self.0 != expected_method {
            return Err(ResponseShapeError::UnexpectedMethodName {
                expected: expected_method.to_owned(),
                actual: self.0,
            });
        }
        if self.2 != expected_call_id {
            return Err(ResponseShapeError::UnexpectedCallId {
                expected: expected_call_id.to_owned(),
                actual: self.2,
            });
        }
        Ok(self)
    }

    /// Convert the response payload into a `Result`.
    ///
    /// # Errors
    ///
    /// Returns the decoded JMAP error when the payload is an error response.
    pub fn into_result(self) -> Result<T, JmapError> {
        self.1.into_result()
    }
}

/// Either a successful method payload or a server-side JMAP error.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MethodResponseData<T> {
    /// Server-side error data.
    Error(JmapError),
    /// Successful method payload.
    Success(T),
}

impl<T> MethodResponseData<T> {
    /// Convert a method response payload into `Result<T, JmapError>`.
    ///
    /// # Errors
    ///
    /// Returns the decoded JMAP error when the payload is an error response.
    pub fn into_result(self) -> Result<T, JmapError> {
        match self {
            Self::Error(error) => Err(error),
            Self::Success(value) => Ok(value),
        }
    }
}

/// Generic JMAP error object.
#[derive(Clone, Debug, Deserialize, Error, PartialEq, Serialize)]
#[error("{error_type}: {description}")]
pub struct JmapError {
    /// JMAP error type URI fragment.
    #[serde(rename = "type")]
    pub error_type: String,
    /// Optional property names referenced by the error.
    pub properties: Option<Vec<String>>,
    /// Human-readable server description.
    #[serde(default)]
    pub description: String,
}

/// Structural errors in a decoded JMAP response envelope.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum ResponseShapeError {
    /// No method responses were returned.
    #[error("response did not contain any method responses")]
    NoMethodResponses,
    /// More than one method response was returned unexpectedly.
    #[error("response contained {count} method responses")]
    UnexpectedMethodCount {
        /// Number of method responses present.
        count: usize,
    },
    /// The method name did not match the expected one.
    #[error("expected method '{expected}' but received '{actual}'")]
    UnexpectedMethodName {
        /// Expected method name.
        expected: String,
        /// Actual method name.
        actual: String,
    },
    /// The call ID did not match the expected one.
    #[error("expected call ID '{expected}' but received '{actual}'")]
    UnexpectedCallId {
        /// Expected call ID.
        expected: String,
        /// Actual call ID.
        actual: String,
    },
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rstest::rstest;

    use super::{
        JmapError, JmapResponse, MethodResponse, MethodResponseData, RequestObject,
        ResponseShapeError, SessionResource,
    };

    #[rstest]
    fn request_builder_serializes_tuple_invocation() {
        let request = RequestObject::new([super::CORE_CAPABILITY])
            .with_method_call("Core/echo", &serde_json::json!({"hello": true}), "call-1")
            .expect("request should serialize");

        assert_eq!(request.method_calls.len(), 1);
        assert_eq!(
            request
                .method_calls
                .first()
                .map(|invocation| invocation.0.as_str()),
            Some("Core/echo")
        );
    }

    #[rstest]
    fn single_response_validation_checks_method_and_call_id() {
        let response = JmapResponse {
            method_responses: vec![MethodResponse(
                "Mailbox/get".to_owned(),
                MethodResponseData::Success(serde_json::json!({"ok": true})),
                "call-1".to_owned(),
            )],
            session_state: "state-1".to_owned(),
            created_ids: None,
        };

        let method = response
            .into_single_response("Mailbox/get", "call-1")
            .expect("response should match");

        assert_eq!(method.0, "Mailbox/get");
    }

    #[rstest]
    fn method_response_data_maps_server_error() {
        let error = MethodResponseData::<serde_json::Value>::Error(JmapError {
            error_type: "serverFail".to_owned(),
            properties: None,
            description: "boom".to_owned(),
        });

        assert_eq!(
            error.into_result().expect_err("server error expected"),
            JmapError {
                error_type: "serverFail".to_owned(),
                properties: None,
                description: "boom".to_owned(),
            }
        );
    }

    #[rstest]
    fn session_resource_returns_primary_account() {
        let session = SessionResource {
            capabilities: HashMap::new(),
            accounts: HashMap::new(),
            primary_accounts: HashMap::from([(
                super::MAIL_CAPABILITY.to_owned(),
                "account-1".to_owned(),
            )]),
            username: "alice@example.com".to_owned(),
            api_url: "https://example.test/jmap".to_owned(),
            download_url: String::new(),
            upload_url: String::new(),
            event_source_url: None,
            state: "state-1".to_owned(),
        };

        assert_eq!(
            session.primary_account(super::MAIL_CAPABILITY),
            Some("account-1")
        );
    }

    #[rstest]
    fn single_response_validation_rejects_shape_mismatch() {
        let response = JmapResponse {
            method_responses: vec![MethodResponse(
                "Mailbox/get".to_owned(),
                MethodResponseData::Success(serde_json::json!({"ok": true})),
                "call-1".to_owned(),
            )],
            session_state: "state-1".to_owned(),
            created_ids: None,
        };

        assert_eq!(
            response
                .into_single_response("Email/get", "call-1")
                .expect_err("method mismatch expected"),
            ResponseShapeError::UnexpectedMethodName {
                expected: "Email/get".to_owned(),
                actual: "Mailbox/get".to_owned(),
            }
        );
    }
}
