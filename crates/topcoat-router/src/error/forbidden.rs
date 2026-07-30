use http::StatusCode;
use topcoat_core::context::Cx;
use topcoat_core::error::{HttpErrorResponse, Result};

use crate::{IntoResponse, Response};

/// Builds a forbidden (HTTP 403) response.
#[must_use]
pub fn forbidden() -> ForbiddenError {
    ForbiddenError::new()
}

/// A forbidden response carried as the `Err` variant of a handler `Result`.
#[derive(Debug)]
pub struct ForbiddenError {
    _priv: (),
}

impl ForbiddenError {
    fn new() -> Self {
        Self { _priv: () }
    }
}

impl std::fmt::Display for ForbiddenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("forbidden")
    }
}

impl std::error::Error for ForbiddenError {}

impl HttpErrorResponse for ForbiddenError {
    fn status_code(&self) -> StatusCode {
        StatusCode::FORBIDDEN
    }

    topcoat_core::impl_http_error_response_any!();
}

impl IntoResponse for ForbiddenError {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (self.status_code(), self.response_body()).into_response(cx)
    }
}
