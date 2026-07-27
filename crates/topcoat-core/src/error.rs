use std::fmt::{Debug, Display};

use http::StatusCode;

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

/// Object-safe trait for error types that know their HTTP response.
///
/// Implement this on application error types to participate in topcoat's
/// error response chain. Any type that implements this trait can be
/// converted to `topcoat::Error` via `From` and will render with the
/// correct status code and body.
pub trait HttpErrorResponse: Debug + Display + Send + Sync + 'static {
    /// The HTTP status code this error should render as.
    fn status_code(&self) -> StatusCode;

    /// The response body for this error. Defaults to `Display` output.
    fn response_body(&self) -> String {
        self.to_string()
    }
}

/// Error type used by Topcoat APIs.
///
/// Wraps any type implementing [`HttpErrorResponse`]. The router renders
/// it using the status code and body the concrete error type declares.
///
/// For errors that don't implement `HttpErrorResponse` (serde, tower,
/// etc.), use [`Error::internal`] to wrap them as a 500.
pub struct Error(Box<dyn HttpErrorResponse>);

/// A catch-all wrapper for errors that don't implement `HttpErrorResponse`.
/// Always renders as 500 Internal Server Error. The original message is
/// captured for logging but not exposed in the response body.
#[derive(Debug)]
struct InternalError {
    #[allow(dead_code)]
    source: String,
}

impl Display for InternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("internal server error")
    }
}

impl HttpErrorResponse for InternalError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn response_body(&self) -> String {
        "internal server error".to_owned()
    }
}

impl Error {
    /// The HTTP status code this error renders as.
    #[must_use]
    pub fn status_code(&self) -> StatusCode {
        self.0.status_code()
    }

    /// The response body for this error.
    #[must_use]
    pub fn response_body(&self) -> String {
        self.0.response_body()
    }

    /// Wrap any error as a 500 Internal Server Error.
    ///
    /// Use this for errors that don't implement `HttpErrorResponse`
    /// (serde, tower, header conversion, etc.). The original error
    /// message is captured for logging but NOT exposed in the response.
    pub fn internal(error: impl Display) -> Self {
        Self(Box::new(InternalError {
            source: error.to_string(),
        }))
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

/// Typed errors that know their status code convert directly.
impl<T: HttpErrorResponse> From<T> for Error {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}

// -- Blanket impls for common error types that should be 500 --

impl HttpErrorResponse for std::io::Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn response_body(&self) -> String {
        "internal server error".to_owned()
    }
}

impl HttpErrorResponse for http::Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn response_body(&self) -> String {
        "internal server error".to_owned()
    }
}

impl HttpErrorResponse for tokio::task::JoinError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn response_body(&self) -> String {
        "internal server error".to_owned()
    }
}
