use http::header::LOCATION;
use http::{HeaderValue, StatusCode};
use topcoat_core::context::Cx;
use topcoat_core::error::{HttpErrorResponse, Result};

use crate::{IntoResponse, Response};

/// Builds a temporary (HTTP 307) redirect to `uri`.
#[must_use]
pub fn redirect(uri: &str) -> RedirectError {
    RedirectError::new(StatusCode::TEMPORARY_REDIRECT, uri)
}

/// Builds a permanent (HTTP 308) redirect to `uri`.
#[must_use]
pub fn redirect_permanent(uri: &str) -> RedirectError {
    RedirectError::new(StatusCode::PERMANENT_REDIRECT, uri)
}

/// A redirect response carried as the `Err` variant of a handler `Result`.
#[derive(Debug)]
pub struct RedirectError {
    status: StatusCode,
    location: HeaderValue,
}

impl RedirectError {
    fn new(status: StatusCode, uri: &str) -> Self {
        Self {
            status,
            location: HeaderValue::try_from(uri).expect("redirect uri is not a valid header value"),
        }
    }
}

impl std::fmt::Display for RedirectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("redirect")
    }
}

impl std::error::Error for RedirectError {}

impl HttpErrorResponse for RedirectError {
    fn status_code(&self) -> StatusCode {
        self.status
    }

    topcoat_core::impl_http_error_response_any!();
}

impl IntoResponse for RedirectError {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (self.status, ([(LOCATION, self.location)], ())).into_response(cx)
    }
}

/// Builds a "see other" (HTTP 303) redirect to `uri`.
#[must_use]
pub fn see_other(uri: &str) -> SeeOther {
    SeeOther::new(uri)
}

/// A "see other" (HTTP 303) redirect response.
#[derive(Debug)]
pub struct SeeOther {
    location: HeaderValue,
}

impl SeeOther {
    fn new(uri: &str) -> Self {
        Self {
            location: HeaderValue::try_from(uri).expect("redirect uri is not a valid header value"),
        }
    }
}

impl IntoResponse for SeeOther {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (StatusCode::SEE_OTHER, ([(LOCATION, self.location)], ())).into_response(cx)
    }
}
