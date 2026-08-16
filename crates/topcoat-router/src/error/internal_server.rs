use http::StatusCode;
use topcoat_core::context::Cx;
use topcoat_core::error::{HttpErrorResponse, Result};

use crate::response::{IntoResponse, Response};

/// Builds an internal-server-error (HTTP 500) response.
///
/// Use this when a handler needs to turn an unexpected application error
/// into a response without exposing the underlying error to the client.
///
/// # Examples
///
/// ```rust
/// use topcoat::{Result, context::Cx, router::error::internal_server_error};
///
/// async fn load_dashboard(cx: &Cx) -> Result<&'static str> {
///     Err(internal_server_error("something broke").into())
/// }
/// ```
pub fn internal_server_error(description: impl Into<String>) -> InternalServerError {
    InternalServerError {
        description: description.into(),
    }
}

/// An internal-server-error response carried as the `Err` variant of a handler `Result`.
///
/// Construct one with [`internal_server_error`].
#[derive(Debug)]
pub struct InternalServerError {
    #[allow(dead_code)]
    description: String,
}

impl std::fmt::Display for InternalServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("internal server error")
    }
}

impl std::error::Error for InternalServerError {}

impl HttpErrorResponse for InternalServerError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn response_body(&self) -> String {
        "internal server error".to_owned()
    }

    topcoat_core::impl_http_error_response_any!();
}

impl IntoResponse for InternalServerError {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (self.status_code(), self.response_body()).into_response(cx)
    }
}
