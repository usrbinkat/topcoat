use http::StatusCode;
use topcoat_core::context::Cx;
use topcoat_core::error::{HttpErrorResponse, Result};

use crate::{IntoResponse, Response};

/// Builds an unauthorized (HTTP 401) response.
#[must_use]
pub fn unauthorized() -> UnauthorizedError {
    UnauthorizedError::new()
}

/// An unauthorized response carried as the `Err` variant of a handler `Result`.
#[derive(Debug)]
pub struct UnauthorizedError {
    _priv: (),
}

impl UnauthorizedError {
    fn new() -> Self {
        Self { _priv: () }
    }
}

impl std::fmt::Display for UnauthorizedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("unauthorized")
    }
}

impl std::error::Error for UnauthorizedError {}

impl HttpErrorResponse for UnauthorizedError {
    fn status_code(&self) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }

    topcoat_core::impl_http_error_response_any!();
}

impl IntoResponse for UnauthorizedError {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (self.status_code(), self.response_body()).into_response(cx)
    }
}
