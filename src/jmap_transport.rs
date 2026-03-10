//! Host-HTTP transport helpers for JMAP requests.

use serde::de::DeserializeOwned;

use jmap_codec::{
    JmapResponse, MethodResponseData, RequestObject, SessionResource,
    mail::{
        EmailGetArguments, EmailGetResponse, EmailQueryArguments, EmailQueryResponse,
        EmailSetArguments, EmailSetResponse, MailboxGetArguments, MailboxGetResponse,
    },
};

use crate::{
    errors::ToolError,
    host::{Host, HostHttpRequest, HostHttpResponse},
    service::JmapConfig,
};

/// Fetch the JMAP session resource.
pub(crate) fn discover_session<H: Host>(
    host: &H,
    config: &JmapConfig,
) -> Result<SessionResource, ToolError> {
    let response = host.http_request(&HostHttpRequest {
        method: "GET".to_owned(),
        url: format!("{}/.well-known/jmap", config.base_url.trim_end_matches('/')),
        headers_json: "{}".to_owned(),
        body: None,
        timeout_ms: Some(config.timeout_ms),
    })?;

    decode_success_json(&response)
}

/// Execute `Mailbox/get`.
pub(crate) fn mailbox_get<H: Host>(
    host: &H,
    config: &JmapConfig,
    api_url: &str,
    arguments: MailboxGetArguments,
) -> Result<MailboxGetResponse, ToolError> {
    execute_method(
        host,
        &MethodContext {
            config,
            api_url,
            method_name: "Mailbox/get",
        },
        arguments,
    )
}

/// Execute `Email/query`.
pub(crate) fn email_query<H: Host>(
    host: &H,
    config: &JmapConfig,
    api_url: &str,
    arguments: EmailQueryArguments,
) -> Result<EmailQueryResponse, ToolError> {
    execute_method(
        host,
        &MethodContext {
            config,
            api_url,
            method_name: "Email/query",
        },
        arguments,
    )
}

/// Execute `Email/get`.
pub(crate) fn email_get<H: Host>(
    host: &H,
    config: &JmapConfig,
    api_url: &str,
    arguments: EmailGetArguments,
) -> Result<EmailGetResponse, ToolError> {
    execute_method(
        host,
        &MethodContext {
            config,
            api_url,
            method_name: "Email/get",
        },
        arguments,
    )
}

/// Execute `Email/set`.
pub(crate) fn email_set<H: Host>(
    host: &H,
    config: &JmapConfig,
    api_url: &str,
    arguments: EmailSetArguments,
) -> Result<EmailSetResponse, ToolError> {
    execute_method(
        host,
        &MethodContext {
            config,
            api_url,
            method_name: "Email/set",
        },
        arguments,
    )
}

#[derive(Clone, Copy)]
struct MethodContext<'a> {
    config: &'a JmapConfig,
    api_url: &'a str,
    method_name: &'a str,
}

fn execute_method<H: Host, TArgs: serde::Serialize, TResponse: DeserializeOwned>(
    host: &H,
    context: &MethodContext<'_>,
    arguments: TArgs,
) -> Result<TResponse, ToolError> {
    let request = RequestObject::mail()
        .with_method_call(context.method_name, &arguments, "call-0")
        .map_err(ToolError::SerializeOutput)?;
    let body = serde_json::to_vec(&request).map_err(ToolError::SerializeOutput)?;
    let response = host.http_request(&HostHttpRequest {
        method: "POST".to_owned(),
        url: context.api_url.to_owned(),
        headers_json: "{\"content-type\":\"application/json\"}".to_owned(),
        body: Some(body),
        timeout_ms: Some(context.config.timeout_ms),
    })?;
    let envelope: JmapResponse<TResponse> = decode_success_json(&response)?;
    let method = envelope
        .method_responses
        .into_iter()
        .next()
        .ok_or_else(|| {
            ToolError::InvalidResponse("JMAP response had no methodResponses".to_owned())
        })?;

    match method.1 {
        MethodResponseData::Success(payload) => Ok(payload),
        MethodResponseData::Error(error) => Err(ToolError::InvalidResponse(format!(
            "JMAP method returned {}{}",
            error.error_type,
            if error.description.is_empty() {
                String::new()
            } else {
                format!(": {}", error.description)
            }
        ))),
    }
}

fn decode_success_json<T: DeserializeOwned>(response: &HostHttpResponse) -> Result<T, ToolError> {
    if !(200..300).contains(&response.status) {
        return Err(ToolError::UnexpectedHttpStatus {
            status: response.status,
            body: String::from_utf8_lossy(&response.body).into_owned(),
        });
    }

    serde_json::from_slice(&response.body).map_err(ToolError::InvalidJsonResponse)
}
