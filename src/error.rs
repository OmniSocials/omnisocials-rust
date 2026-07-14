//! Error type for the OmniSocials SDK.
//!
//! Every non-2xx API response maps to the most specific variant by status
//! code (mirroring the error hierarchy shared by all OmniSocials SDKs):
//!
//! | Variant                 | Status    |
//! |-------------------------|-----------|
//! | [`Error::Validation`]   | 400 / 422 |
//! | [`Error::Auth`]         | 401       |
//! | [`Error::PermissionDenied`] | 403   |
//! | [`Error::NotFound`]     | 404       |
//! | [`Error::RateLimit`]    | 429       |
//! | [`Error::Server`]       | >= 500    |
//! | [`Error::Api`]          | any other non-2xx |
//!
//! `code` and `message` come from the API error body
//! `{ "error": { "code", "message" } }` when present.

use serde_json::Value;

/// All errors returned by this crate.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A non-2xx API response that does not map to a more specific variant.
    #[error("API error (status {status}): {message}")]
    Api {
        /// HTTP status code of the failed response.
        status: u16,
        /// Machine-readable error code from the API body (e.g. `"validation_error"`).
        code: Option<String>,
        /// Human-readable error message.
        message: String,
        /// The parsed response body, when the API returned JSON.
        body: Option<Value>,
    },

    /// 401: missing or invalid API key. Also returned at construction time
    /// when no API key is provided (code `"missing_api_key"`).
    #[error("authentication error (status {status}): {message}")]
    Auth {
        /// HTTP status code of the failed response.
        status: u16,
        /// Machine-readable error code from the API body.
        code: Option<String>,
        /// Human-readable error message.
        message: String,
        /// The parsed response body, when the API returned JSON.
        body: Option<Value>,
    },

    /// 403: the API key lacks a required scope, or access is forbidden.
    #[error("permission denied (status {status}): {message}")]
    PermissionDenied {
        /// HTTP status code of the failed response.
        status: u16,
        /// Machine-readable error code from the API body.
        code: Option<String>,
        /// Human-readable error message.
        message: String,
        /// The parsed response body, when the API returned JSON.
        body: Option<Value>,
    },

    /// 404: the resource does not exist (or belongs to another workspace).
    #[error("not found (status {status}): {message}")]
    NotFound {
        /// HTTP status code of the failed response.
        status: u16,
        /// Machine-readable error code from the API body.
        code: Option<String>,
        /// Human-readable error message.
        message: String,
        /// The parsed response body, when the API returned JSON.
        body: Option<Value>,
    },

    /// 400 / 422: the request was rejected as invalid.
    #[error("validation error (status {status}): {message}")]
    Validation {
        /// HTTP status code of the failed response.
        status: u16,
        /// Machine-readable error code from the API body.
        code: Option<String>,
        /// Human-readable error message.
        message: String,
        /// The parsed response body, when the API returned JSON.
        body: Option<Value>,
    },

    /// 429: rate limited (the API allows 100 requests per minute).
    /// Retries are automatic; this surfaces only once they are exhausted.
    #[error("rate limited: {message}")]
    RateLimit {
        /// Seconds to wait before retrying, from the `Retry-After` header
        /// (if present).
        retry_after: Option<f64>,
        /// Machine-readable error code from the API body.
        code: Option<String>,
        /// Human-readable error message.
        message: String,
        /// The parsed response body, when the API returned JSON.
        body: Option<Value>,
    },

    /// >= 500: the API failed on its side.
    #[error("server error (status {status}): {message}")]
    Server {
        /// HTTP status code of the failed response.
        status: u16,
        /// Machine-readable error code from the API body.
        code: Option<String>,
        /// Human-readable error message.
        message: String,
        /// The parsed response body, when the API returned JSON.
        body: Option<Value>,
    },

    /// Network failure or timeout (the request never produced an HTTP
    /// response). Retried automatically before being surfaced.
    #[error("connection error: {0}")]
    Connection(#[from] reqwest::Error),

    /// Webhook signature verification failed. See
    /// [`crate::webhooks::verify_signature`].
    #[error("webhook verification failed: {0}")]
    WebhookVerification(String),
}

impl Error {
    /// The HTTP status code, for API-response variants (429 for `RateLimit`).
    pub fn status(&self) -> Option<u16> {
        match self {
            Error::Api { status, .. }
            | Error::Auth { status, .. }
            | Error::PermissionDenied { status, .. }
            | Error::NotFound { status, .. }
            | Error::Validation { status, .. }
            | Error::Server { status, .. } => Some(*status),
            Error::RateLimit { .. } => Some(429),
            _ => None,
        }
    }

    /// The machine-readable error code from the API body, when present.
    pub fn code(&self) -> Option<&str> {
        match self {
            Error::Api { code, .. }
            | Error::Auth { code, .. }
            | Error::PermissionDenied { code, .. }
            | Error::NotFound { code, .. }
            | Error::Validation { code, .. }
            | Error::RateLimit { code, .. }
            | Error::Server { code, .. } => code.as_deref(),
            _ => None,
        }
    }

    /// Seconds to wait before retrying, for [`Error::RateLimit`].
    pub fn retry_after(&self) -> Option<f64> {
        match self {
            Error::RateLimit { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Map an HTTP status to the most specific error variant.
    pub(crate) fn from_status(
        status: u16,
        code: Option<String>,
        message: String,
        body: Option<Value>,
        retry_after: Option<f64>,
    ) -> Error {
        match status {
            400 | 422 => Error::Validation { status, code, message, body },
            401 => Error::Auth { status, code, message, body },
            403 => Error::PermissionDenied { status, code, message, body },
            404 => Error::NotFound { status, code, message, body },
            429 => Error::RateLimit { retry_after, code, message, body },
            s if s >= 500 => Error::Server { status, code, message, body },
            _ => Error::Api { status, code, message, body },
        }
    }

    /// Build an error from a non-2xx response body, pulling `code` and
    /// `message` out of `{ "error": { "code", "message" } }` when present.
    pub(crate) fn from_response(status: u16, body_text: &str, retry_after: Option<f64>) -> Error {
        let mut code: Option<String> = None;
        let mut message = format!("Request failed with status {status}.");
        let parsed: Option<Value> = serde_json::from_str(body_text).ok();

        if let Some(parsed_body) = &parsed {
            match parsed_body.get("error") {
                Some(Value::Object(err)) => {
                    if let Some(Value::String(c)) = err.get("code") {
                        code = Some(c.clone());
                    }
                    if let Some(Value::String(m)) = err.get("message") {
                        message = m.clone();
                    }
                }
                Some(Value::String(m)) => message = m.clone(),
                _ => {}
            }
        }

        Error::from_status(status, code, message, parsed, retry_after)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make(status: u16) -> Error {
        Error::from_status(status, Some("some_code".into()), "msg".into(), None, Some(7.0))
    }

    #[test]
    fn maps_statuses_to_variants() {
        assert!(matches!(make(400), Error::Validation { status: 400, .. }));
        assert!(matches!(make(422), Error::Validation { status: 422, .. }));
        assert!(matches!(make(401), Error::Auth { status: 401, .. }));
        assert!(matches!(make(403), Error::PermissionDenied { status: 403, .. }));
        assert!(matches!(make(404), Error::NotFound { status: 404, .. }));
        assert!(matches!(make(429), Error::RateLimit { retry_after: Some(r), .. } if r == 7.0));
        assert!(matches!(make(500), Error::Server { status: 500, .. }));
        assert!(matches!(make(503), Error::Server { status: 503, .. }));
        assert!(matches!(make(418), Error::Api { status: 418, .. }));
    }

    #[test]
    fn accessors_expose_status_code_retry_after() {
        let err = make(429);
        assert_eq!(err.status(), Some(429));
        assert_eq!(err.code(), Some("some_code"));
        assert_eq!(err.retry_after(), Some(7.0));

        let err = make(404);
        assert_eq!(err.status(), Some(404));
        assert_eq!(err.retry_after(), None);
    }

    #[test]
    fn parses_error_body_envelope() {
        let err = Error::from_response(
            422,
            r#"{"error":{"code":"validation_error","message":"content is required"}}"#,
            None,
        );
        assert!(matches!(&err, Error::Validation { status: 422, .. }));
        assert_eq!(err.code(), Some("validation_error"));
        assert_eq!(err.to_string(), "validation error (status 422): content is required");
    }

    #[test]
    fn tolerates_non_json_error_body() {
        let err = Error::from_response(413, "<html>Request Entity Too Large</html>", None);
        assert!(matches!(&err, Error::Api { status: 413, body: None, .. }));
        assert_eq!(err.to_string(), "API error (status 413): Request failed with status 413.");
    }
}
