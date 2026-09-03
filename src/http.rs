//! HTTP response helpers and the error model.
//!
//! `handle_http_request` must never return `Err` for domain failures (the host
//! turns that into a plain-text 500), so every error becomes a JSON response
//! here. The wire shape is the Go plugin's exactly: `{"code": "...",
//! "message": "..."}` with codes `INVALID_INPUT`, `NOT_FOUND`, `CONFLICT`,
//! `FORBIDDEN`, `INTERNAL_ERROR`, and parameter-error messages matching the Go
//! helpers verbatim so API consumers see no difference.

use std::collections::HashMap;

use gameap_plugin_sdk::proto::gameap::plugin as pb;
use serde::Serialize;

use crate::host_api::HostApiError;

pub type ApiResult = Result<pb::HttpResponse, ApiError>;

pub const ERR_INVALID_INPUT: &str = "INVALID_INPUT";
pub const ERR_NOT_FOUND: &str = "NOT_FOUND";
pub const ERR_CONFLICT: &str = "CONFLICT";
/// Declared by the Go plugin; no handler emits it today. Kept for parity.
pub const ERR_FORBIDDEN: &str = "FORBIDDEN";
pub const ERR_INTERNAL: &str = "INTERNAL_ERROR";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    pub status: i32,
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    pub fn new(status: i32, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(400, ERR_INVALID_INPUT, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(404, ERR_NOT_FOUND, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(409, ERR_CONFLICT, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(500, ERR_INTERNAL, message)
    }

    pub fn into_response(self) -> pb::HttpResponse {
        #[derive(Serialize)]
        struct ErrorBody {
            code: &'static str,
            message: String,
        }
        json_response(
            self.status,
            &ErrorBody {
                code: self.code,
                message: self.message,
            },
        )
    }
}

impl From<HostApiError> for ApiError {
    // The Go plugin surfaced every host/daemon failure as a 500 with the
    // underlying error text; keep that mapping instead of firewall's 502s.
    fn from(err: HostApiError) -> Self {
        Self::internal(err.into_message())
    }
}

pub fn json_response<T: Serialize>(status: i32, value: &T) -> pb::HttpResponse {
    match serde_json::to_vec(value) {
        Ok(body) => pb::HttpResponse {
            status_code: status,
            headers: json_headers(),
            body,
            file: None,
        },
        Err(_) => pb::HttpResponse {
            status_code: 500,
            headers: json_headers(),
            body: br#"{"code":"INTERNAL_ERROR","message":"failed to serialize response"}"#.to_vec(),
            file: None,
        },
    }
}

fn json_headers() -> HashMap<String, String> {
    HashMap::from([("Content-Type".to_string(), "application/json".to_string())])
}

/// Parses a JSON body with the Go user-handlers' terse error message.
pub fn parse_json_body<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T, ApiError> {
    serde_json::from_slice(body).map_err(|_| ApiError::bad_request("invalid request body"))
}

/// Parses a JSON body with the Go node-handlers' detailed error message.
pub fn parse_json_body_verbose<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T, ApiError> {
    serde_json::from_slice(body)
        .map_err(|err| ApiError::bad_request(format!("invalid request body: {err}")))
}

/// Go `ParseUint64Param`: message parity matters, not just the status.
pub fn parse_u64_param(params: &HashMap<String, String>, key: &str) -> Result<u64, ApiError> {
    let val = params
        .get(key)
        .ok_or_else(|| ApiError::bad_request(format!("missing parameter: {key}")))?;
    val.parse::<u64>()
        .map_err(|_| ApiError::bad_request(format!("invalid {key}: must be a number")))
}

/// Go `ParseIntParam` (used for the SSH key index; may be negative).
pub fn parse_int_param(params: &HashMap<String, String>, key: &str) -> Result<i64, ApiError> {
    let val = params
        .get(key)
        .ok_or_else(|| ApiError::bad_request(format!("missing parameter: {key}")))?;
    val.parse::<i64>()
        .map_err(|_| ApiError::bad_request(format!("invalid {key}: must be a number")))
}

/// Go `GetStringParam`: absent and empty are the same failure.
pub fn get_string_param<'a>(
    params: &'a HashMap<String, String>,
    key: &str,
) -> Result<&'a str, ApiError> {
    match params.get(key).map(String::as_str) {
        Some(v) if !v.is_empty() => Ok(v),
        _ => Err(ApiError::bad_request(format!("missing parameter: {key}"))),
    }
}

/// First value of a query parameter, or "" (Go `getQueryParam`).
pub fn query_param<'a>(query: &'a HashMap<String, pb::QueryParamValues>, key: &str) -> &'a str {
    query
        .get(key)
        .and_then(|v| v.values.first())
        .map(String::as_str)
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_body_shape() {
        let resp = ApiError::conflict("user already exists").into_response();
        assert_eq!(resp.status_code, 409);
        assert_eq!(
            String::from_utf8(resp.body).unwrap(),
            r#"{"code":"CONFLICT","message":"user already exists"}"#
        );
        let resp = ApiError::not_found("route not found").into_response();
        assert_eq!(
            String::from_utf8(resp.body).unwrap(),
            r#"{"code":"NOT_FOUND","message":"route not found"}"#
        );
    }

    #[test]
    fn param_error_messages_match_go() {
        let mut params = HashMap::new();
        assert_eq!(
            parse_u64_param(&params, "serverId").unwrap_err().message,
            "missing parameter: serverId"
        );
        params.insert("serverId".into(), "abc".into());
        assert_eq!(
            parse_u64_param(&params, "serverId").unwrap_err().message,
            "invalid serverId: must be a number"
        );
        params.insert("username".into(), String::new());
        assert_eq!(
            get_string_param(&params, "username").unwrap_err().message,
            "missing parameter: username"
        );
        params.insert("index".into(), "-1".into());
        assert_eq!(parse_int_param(&params, "index").unwrap(), -1);
    }

    #[test]
    fn json_body_parsing() {
        assert!(parse_json_body::<serde_json::Value>(b"{oops").is_err());
        assert_eq!(
            parse_json_body::<serde_json::Value>(b"{oops")
                .unwrap_err()
                .message,
            "invalid request body"
        );
        assert!(
            parse_json_body_verbose::<serde_json::Value>(b"")
                .unwrap_err()
                .message
                .starts_with("invalid request body: ")
        );
    }
}
