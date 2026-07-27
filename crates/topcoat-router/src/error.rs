#![doc = include_str!("../docs/error.md")]

mod bad_request;
mod forbidden;
mod internal_server;
mod method_not_allowed;
mod not_found;
mod redirect;
mod unauthorized;

pub use bad_request::*;
pub use forbidden::*;
pub use internal_server::*;
pub use method_not_allowed::*;
pub use not_found::*;
pub use redirect::*;
pub use unauthorized::*;

use http::StatusCode;

use crate::{Body, IntoResponse, Response};
use topcoat_core::context::Cx;
use topcoat_core::error::{Error, Result};

/// Renders any [`IntoResponse`] value into a [`Response`], falling back to the
/// error's response if conversion fails. This is the terminal conversion the
/// router applies to a handler's return value.
pub(crate) fn respond(cx: &Cx, value: impl IntoResponse) -> Response {
    value
        .into_response(cx)
        .unwrap_or_else(|error| error_into_response(cx, error))
}

/// Maps any error onto its HTTP status code and body using the
/// [`HttpErrorResponse`] trait. No hardcoded type list — any application
/// error that implements the trait participates automatically.
fn error_into_response(cx: &Cx, error: Error) -> Response {
    let status = error.status_code();
    let body = error.response_body();
    into_response_or_500(cx, (status, body))
}

/// Renders an error response, falling back to a bare 500.
fn into_response_or_500(cx: &Cx, value: impl IntoResponse) -> Response {
    value.into_response(cx).unwrap_or_else(|_| {
        let mut response = Response::new(Body::from("internal server error"));
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        response
    })
}

/// Renders the contained value, or the framework error response on `Err`.
impl<T> IntoResponse for Result<T>
where
    T: IntoResponse,
{
    fn into_response(self, cx: &Cx) -> Result<Response> {
        match self {
            Ok(value) => value.into_response(cx),
            Err(error) => Ok(error_into_response(cx, error)),
        }
    }
}

/// Renders an error by mapping it onto its HTTP status code.
impl IntoResponse for Error {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        Ok(error_into_response(cx, self))
    }
}

/// Converts an absent or failed value into a router error response.
///
/// Implemented for [`Option`] (where `None` becomes the configured error)
/// and [`core::result::Result`] (where any `Err` is replaced, discarding the
/// original error). Designed to be combined with `?` so a handler can return a
/// redirect, not-found, unauthorized, forbidden, or bad-request response when
/// required state is missing or invalid.
pub trait RouterErrorExt {
    /// The success type produced when the value is present.
    type T;

    fn ok_or_redirect(self, uri: &str) -> Result<Self::T, RedirectError>;
    fn ok_or_redirect_permanent(self, uri: &str) -> Result<Self::T, RedirectError>;
    fn ok_or_not_found(self) -> Result<Self::T, NotFoundError>;
    fn ok_or_unauthorized(self) -> Result<Self::T, UnauthorizedError>;
    fn ok_or_forbidden(self) -> Result<Self::T, ForbiddenError>;
    fn ok_or_bad_request(self, description: impl Into<String>) -> Result<Self::T, BadRequestError>;
}

impl<T> RouterErrorExt for Option<T> {
    type T = T;

    fn ok_or_redirect(self, uri: &str) -> Result<Self::T, RedirectError> {
        self.ok_or_else(|| redirect(uri))
    }

    fn ok_or_redirect_permanent(self, uri: &str) -> Result<Self::T, RedirectError> {
        self.ok_or_else(|| redirect_permanent(uri))
    }

    fn ok_or_not_found(self) -> Result<Self::T, NotFoundError> {
        self.ok_or_else(not_found)
    }

    fn ok_or_unauthorized(self) -> Result<Self::T, UnauthorizedError> {
        self.ok_or_else(unauthorized)
    }

    fn ok_or_forbidden(self) -> Result<Self::T, ForbiddenError> {
        self.ok_or_else(forbidden)
    }

    fn ok_or_bad_request(self, description: impl Into<String>) -> Result<Self::T, BadRequestError> {
        self.ok_or_else(|| bad_request(description))
    }
}

impl<T, E> RouterErrorExt for Result<T, E> {
    type T = T;

    fn ok_or_redirect(self, uri: &str) -> Result<Self::T, RedirectError> {
        self.map_err(|_| redirect(uri))
    }

    fn ok_or_redirect_permanent(self, uri: &str) -> Result<Self::T, RedirectError> {
        self.map_err(|_| redirect_permanent(uri))
    }

    fn ok_or_not_found(self) -> Result<Self::T, NotFoundError> {
        self.map_err(|_| not_found())
    }

    fn ok_or_unauthorized(self) -> Result<Self::T, UnauthorizedError> {
        self.map_err(|_| unauthorized())
    }

    fn ok_or_forbidden(self) -> Result<Self::T, ForbiddenError> {
        self.map_err(|_| forbidden())
    }

    fn ok_or_bad_request(self, description: impl Into<String>) -> Result<Self::T, BadRequestError> {
        self.map_err(|_| bad_request(description))
    }
}
