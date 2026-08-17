//! Outbound HTTP client with context propagation and health tracking.
//!
//! `OutboundClient` wraps `reqwest::Client` and bridges topcoat's
//! inbound request context to outbound HTTP calls:
//!
//! - **Request ID propagation**: reads `X-Request-Id` from the inbound
//!   context and injects it into outbound request headers for distributed
//!   tracing correlation.
//! - **Health tracking**: per-host success/failure counts via DashMap,
//!   exposed for health check integration.
//! - **Shared connection pool**: one client instance registered in app
//!   context, reused across all outbound calls (resolvers, probes,
//!   federation, crawl notifications).
//!
//! reqwest handles retry (scoped budgets + classifiers), TLS (rustls),
//! connection pooling, redirect policy, proxy, and timeout. This module
//! adds what reqwest can't do: context propagation and observability.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use dashmap::DashMap;

/// Per-host health state tracked across outbound requests.
#[derive(Debug)]
pub struct HostHealth {
    pub successes: AtomicU64,
    pub failures: AtomicU64,
}

impl HostHealth {
    fn new() -> Self {
        Self {
            successes: AtomicU64::new(0),
            failures: AtomicU64::new(0),
        }
    }

    /// Failure rate as a fraction (0.0 = all success, 1.0 = all failure).
    /// Returns 0.0 if no requests have been made.
    pub fn failure_rate(&self) -> f64 {
        let s = self.successes.load(Ordering::Relaxed);
        let f = self.failures.load(Ordering::Relaxed);
        let total = s + f;
        if total == 0 { 0.0 } else { f as f64 / total as f64 }
    }

    /// Total requests made to this host.
    pub fn total(&self) -> u64 {
        self.successes.load(Ordering::Relaxed)
            + self.failures.load(Ordering::Relaxed)
    }
}

/// Configuration for building an `OutboundClient`.
pub struct OutboundClientBuilder {
    timeout: Duration,
    connect_timeout: Duration,
    redirect_limit: usize,
    user_agent: String,
    request_id_header: String,
}

impl Default for OutboundClientBuilder {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            redirect_limit: 3,
            user_agent: format!("topcoat/{}", env!("CARGO_PKG_VERSION")),
            request_id_header: "x-request-id".to_string(),
        }
    }
}

impl OutboundClientBuilder {
    /// Set the per-request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the connection timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set the redirect follow limit.
    pub fn redirect_limit(mut self, limit: usize) -> Self {
        self.redirect_limit = limit;
        self
    }

    /// Set the User-Agent header.
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = ua.into();
        self
    }

    /// Set the header name used for request ID propagation.
    /// Default: `x-request-id`.
    pub fn request_id_header(mut self, name: impl Into<String>) -> Self {
        self.request_id_header = name.into();
        self
    }

    /// Build the `OutboundClient`.
    pub fn build(self) -> OutboundClient {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .connect_timeout(self.connect_timeout)
            .redirect(reqwest::redirect::Policy::limited(self.redirect_limit))
            .user_agent(&self.user_agent)
            .build()
            .expect("failed to build reqwest outbound client");

        OutboundClient {
            client,
            host_health: Arc::new(DashMap::new()),
            request_id_header: self.request_id_header,
        }
    }
}

/// Shared outbound HTTP client with context propagation and health tracking.
///
/// Register as `Arc<OutboundClient>` in app context at server startup.
/// Handlers retrieve it via `app_context::<Arc<OutboundClient>>(cx)`.
///
/// ```ignore
/// // Server startup:
/// let outbound = Arc::new(OutboundClient::new());
/// builder = builder.app_context(outbound);
///
/// // In a handler:
/// let client = app_context::<Arc<OutboundClient>>(cx);
/// let resp = client.get(cx, "https://plc.directory/did:plc:xyz").await?;
/// ```
pub struct OutboundClient {
    client: reqwest::Client,
    host_health: Arc<DashMap<String, HostHealth>>,
    request_id_header: String,
}

impl OutboundClient {
    /// Create with default configuration.
    pub fn new() -> Self {
        OutboundClientBuilder::default().build()
    }

    /// Create with custom configuration.
    pub fn builder() -> OutboundClientBuilder {
        OutboundClientBuilder::default()
    }

    /// Send a GET request with context propagation.
    ///
    /// Injects the inbound request ID into the outbound `X-Request-Id`
    /// header. Tracks success/failure per host.
    pub async fn get(
        &self,
        cx: &topcoat_core::context::Cx,
        url: &str,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let mut req = self.client.get(url);
        req = self.inject_request_id(cx, req);
        let resp = req.send().await;
        self.track_health(url, resp.is_ok());
        resp
    }

    /// Send a POST request with JSON body and context propagation.
    pub async fn post_json(
        &self,
        cx: &topcoat_core::context::Cx,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let mut req = self.client.post(url).json(body);
        req = self.inject_request_id(cx, req);
        let resp = req.send().await;
        self.track_health(url, resp.is_ok());
        resp
    }

    /// Send a POST request with raw bytes and context propagation.
    pub async fn post_bytes(
        &self,
        cx: &topcoat_core::context::Cx,
        url: &str,
        body: Vec<u8>,
        content_type: &str,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let mut req = self.client.post(url)
            .header("content-type", content_type)
            .body(body);
        req = self.inject_request_id(cx, req);
        let resp = req.send().await;
        self.track_health(url, resp.is_ok());
        resp
    }

    /// Access the underlying reqwest::Client for advanced use cases.
    ///
    /// Prefer the typed methods (get, post_json, post_bytes) when possible
    /// — they handle context propagation and health tracking automatically.
    /// Direct client access bypasses both.
    pub fn inner(&self) -> &reqwest::Client {
        &self.client
    }

    /// Per-host health snapshot for health check integration.
    ///
    /// Returns (host, failure_rate, total_requests) for each tracked host.
    /// Wire this into your health endpoint to report outbound dependency status.
    pub fn health(&self) -> Vec<(String, f64, u64)> {
        self.host_health
            .iter()
            .map(|entry| {
                let host = entry.key().clone();
                let rate = entry.value().failure_rate();
                let total = entry.value().total();
                (host, rate, total)
            })
            .collect()
    }

    /// Check if a specific host is healthy (failure rate below threshold).
    pub fn is_host_healthy(&self, host: &str, max_failure_rate: f64) -> bool {
        self.host_health
            .get(host)
            .map(|h| h.failure_rate() < max_failure_rate)
            .unwrap_or(true) // unknown host = healthy (no evidence of failure)
    }

    /// Reset health counters for all hosts.
    pub fn reset_health(&self) {
        self.host_health.clear();
    }

    // -- Internal helpers -----------------------------------------------------

    fn inject_request_id(
        &self,
        cx: &topcoat_core::context::Cx,
        req: reqwest::RequestBuilder,
    ) -> reqwest::RequestBuilder {
        // Read request ID from inbound context if available.
        // The request ID is stored as a String in request context
        // by the server's request_id layer.
        if let Some(request_id) = topcoat_core::context::try_request_context::<String>(cx) {
            req.header(&self.request_id_header, request_id.as_str())
        } else {
            req
        }
    }

    fn track_health(&self, url: &str, success: bool) {
        let host = extract_host(url);
        let entry = self.host_health
            .entry(host)
            .or_insert_with(HostHealth::new);
        if success {
            entry.successes.fetch_add(1, Ordering::Relaxed);
        } else {
            entry.failures.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl Default for OutboundClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract the host portion from a URL for health tracking.
fn extract_host(url: &str) -> String {
    // Fast path: find :// then next /
    if let Some(after_scheme) = url.find("://").map(|i| i + 3) {
        let rest = &url[after_scheme..];
        let end = rest.find('/').unwrap_or(rest.len());
        rest[..end].to_string()
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_host_from_url() {
        assert_eq!(extract_host("https://plc.directory/did:plc:abc"), "plc.directory");
        assert_eq!(extract_host("http://localhost:5000/v2/"), "localhost:5000");
        assert_eq!(extract_host("https://example.com"), "example.com");
    }

    #[test]
    fn host_health_tracking() {
        let health = HostHealth::new();
        assert_eq!(health.failure_rate(), 0.0);
        assert_eq!(health.total(), 0);

        health.successes.fetch_add(8, Ordering::Relaxed);
        health.failures.fetch_add(2, Ordering::Relaxed);
        assert_eq!(health.total(), 10);
        assert!((health.failure_rate() - 0.2).abs() < 0.001);
    }

    #[test]
    fn is_host_healthy_threshold() {
        let client = OutboundClient::new();
        // Unknown host = healthy
        assert!(client.is_host_healthy("unknown.com", 0.5));

        // Track some failures
        for _ in 0..10 {
            client.track_health("http://sick.com/path", false);
        }
        assert!(!client.is_host_healthy("sick.com", 0.5));

        // Track successes to bring rate down
        for _ in 0..90 {
            client.track_health("http://sick.com/path", true);
        }
        assert!(client.is_host_healthy("sick.com", 0.5));
    }

    #[test]
    fn builder_defaults() {
        let client = OutboundClient::builder()
            .timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(2))
            .redirect_limit(5)
            .user_agent("test/1.0")
            .request_id_header("x-trace-id")
            .build();
        assert!(client.health().is_empty());
    }

    #[test]
    fn reset_health_clears_all() {
        let client = OutboundClient::new();
        client.track_health("http://a.com/", true);
        client.track_health("http://b.com/", false);
        assert_eq!(client.health().len(), 2);
        client.reset_health();
        assert!(client.health().is_empty());
    }
}
