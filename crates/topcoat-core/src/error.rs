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

    /// Additional headers to include in the error response.
    ///
    /// Override this to attach protocol-required headers to error responses,
    /// e.g. `Allow` on 405, `WWW-Authenticate` on 401, `Retry-After` on 429.
    /// The default returns an empty map.
    fn error_headers(&self) -> http::HeaderMap {
        http::HeaderMap::new()
    }

    /// Upcast to `Any` for downcasting. Provided automatically.
    #[doc(hidden)]
    fn as_any(&self) -> &dyn std::any::Any;

    /// Upcast to mutable `Any` for downcasting. Provided automatically.
    #[doc(hidden)]
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;

    /// Move into `Box<dyn Any>` for owned downcasting. Provided automatically.
    #[doc(hidden)]
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any>;
}

/// Generates the three `Any` upcast methods required by [`HttpErrorResponse`].
///
/// Every `impl HttpErrorResponse for MyError` must include these. The macro
/// generates them from the implementing type.
#[macro_export]
macro_rules! impl_http_error_response_any {
    () => {
        fn as_any(&self) -> &dyn ::std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn ::std::any::Any {
            self
        }
        fn into_any(self: Box<Self>) -> Box<dyn ::std::any::Any> {
            self
        }
    };
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

    impl_http_error_response_any!();
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

    /// Additional headers for the error response.
    #[must_use]
    pub fn error_headers(&self) -> http::HeaderMap {
        self.0.error_headers()
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

    /// Downcast this error by reference to a concrete type.
    #[must_use]
    pub fn downcast_ref<E: HttpErrorResponse>(&self) -> Option<&E> {
        self.0.as_any().downcast_ref::<E>()
    }

    /// Downcast this error by mutable reference to a concrete type.
    pub fn downcast_mut<E: HttpErrorResponse>(&mut self) -> Option<&mut E> {
        self.0.as_any_mut().downcast_mut::<E>()
    }

    /// Attempt to downcast the error to a concrete type.
    ///
    /// # Errors
    ///
    /// Returns `Err(Self)` if the stored error is not an instance of `E`.
    pub fn downcast<E: HttpErrorResponse>(self) -> Result<E, Self> {
        if self.0.as_any().is::<E>() {
            Ok(*self.0.into_any().downcast::<E>().unwrap())
        } else {
            Err(self)
        }
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

impl From<Error> for Box<dyn std::error::Error + Send + Sync + 'static> {
    fn from(error: Error) -> Self {
        Box::new(ErrorWrapper(error))
    }
}

/// Wrapper to impl std::error::Error for Error, needed for the
/// Box<dyn Error + Send + Sync> conversion above.
#[derive(Debug)]
struct ErrorWrapper(Error);

impl Display for ErrorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl std::error::Error for ErrorWrapper {}

// -- Blanket impls for common error types that should be 500 --

impl HttpErrorResponse for std::io::Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn response_body(&self) -> String {
        "internal server error".to_owned()
    }

    impl_http_error_response_any!();
}

impl HttpErrorResponse for http::Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn response_body(&self) -> String {
        "internal server error".to_owned()
    }

    impl_http_error_response_any!();
}

impl HttpErrorResponse for tokio::task::JoinError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn response_body(&self) -> String {
        "internal server error".to_owned()
    }

    impl_http_error_response_any!();
}

impl HttpErrorResponse for http::header::InvalidHeaderValue {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn response_body(&self) -> String {
        "internal server error".to_owned()
    }

    impl_http_error_response_any!();
}
