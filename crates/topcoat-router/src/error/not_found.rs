use http::StatusCode;
use topcoat_core::context::Cx;
use topcoat_core::error::{HttpErrorResponse, Result};

use crate::response::{IntoResponse, Response};

/// Builds a not-found (HTTP 404) response.
///
/// # Examples
///
/// ```rust
/// # struct User;
/// # async fn lookup(_cx: &Cx, _id: u64) -> Option<User> { None }
/// use topcoat::{Result, context::Cx, router::error::not_found};
///
/// async fn fetch_user(cx: &Cx, id: u64) -> Result<User> {
///     let Some(user) = lookup(cx, id).await else {
///         return Err(not_found().into());
///     };
///     Ok(user)
/// }
/// ```
#[must_use]
pub fn not_found() -> NotFoundError {
    NotFoundError::new()
}

/// A not-found response carried as the `Err` variant of a handler `Result`.
///
/// Construct one with [`not_found`], or derive one from an `Option` /
/// `Result` via [`RouterErrorExt`](crate::error::RouterErrorExt).
#[derive(Debug)]
pub struct NotFoundError {
    _priv: (),
}

impl NotFoundError {
    fn new() -> Self {
        Self { _priv: () }
    }
}

impl std::fmt::Display for NotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("not found")
    }
}

impl std::error::Error for NotFoundError {}

impl HttpErrorResponse for NotFoundError {
    fn status_code(&self) -> StatusCode {
        StatusCode::NOT_FOUND
    }

    topcoat_core::impl_http_error_response_any!();
}

impl IntoResponse for NotFoundError {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (self.status_code(), self.response_body()).into_response(cx)
    }
}
