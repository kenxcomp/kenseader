use bytes::Bytes;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, REFERER, USER_AGENT};
use reqwest::{Client, Proxy};
use url::Url;
use uuid::Uuid;

use super::parser::{parse_feed, ParsedFeed};
use crate::config::AppConfig;
use crate::{Error, Result};

const RSSHUB_SCHEME: &str = "rsshub";
const MAX_FEED_BYTES: usize = 5 * 1024 * 1024;
const MAX_RETRIES: u32 = 3;
const INITIAL_RETRY_DELAY_MS: u64 = 500;

// Rotating User-Agent pool - mimics different browsers for better compatibility
static USER_AGENT_INDEX: AtomicUsize = AtomicUsize::new(0);
const USER_AGENTS: &[&str] = &[
    // Chrome on macOS
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    // Chrome on Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    // Firefox on macOS
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0",
    // Firefox on Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
    // Safari on macOS
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15",
    // Edge on Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
];

/// Get the next User-Agent in rotation
fn next_user_agent() -> &'static str {
    let index = USER_AGENT_INDEX.fetch_add(1, Ordering::Relaxed) % USER_AGENTS.len();
    USER_AGENTS[index]
}

/// Feed fetcher with HTTP client and RSSHub support
pub struct FeedFetcher {
    client: Client,
    rsshub_base_url: String,
    rsshub_access_key: Option<String>,
}

impl FeedFetcher {
    /// Create a new feed fetcher with configuration
    pub fn new(config: &AppConfig) -> Result<Self> {
        let client = Self::build_client(config.sync.request_timeout_secs, &config.sync.proxy_url)?;

        Ok(Self {
            client,
            rsshub_base_url: config.rsshub.base_url.clone(),
            rsshub_access_key: config.rsshub.access_key.clone(),
        })
    }

    /// Build HTTP client with optional proxy
    fn build_client(timeout_secs: u64, proxy_url: &Option<String>) -> Result<Client> {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .gzip(true)
            .deflate(true)
            .brotli(true)
            .redirect(reqwest::redirect::Policy::limited(10));

        // Configure proxy if provided
        if let Some(ref proxy) = proxy_url {
            let proxy = Proxy::all(proxy)
                .map_err(|e| Error::Config(format!("Invalid proxy URL: {}", e)))?;
            builder = builder.proxy(proxy);
            tracing::info!("Using HTTP proxy for feed fetching");
        }

        builder.build().map_err(|e| Error::Http(e))
    }

    /// Build browser-like headers for a request
    fn build_headers(user_agent: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT,
            HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,application/rss+xml,application/atom+xml,*/*;q=0.8"
            )
        );
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("en-US,en;q=0.9,zh-CN;q=0.8,zh;q=0.7")
        );
        headers.insert(
            ACCEPT_ENCODING,
            HeaderValue::from_static("gzip, deflate, br")
        );
        headers.insert(
            REFERER,
            HeaderValue::from_static("https://www.google.com/")
        );
        if let Ok(ua) = HeaderValue::from_str(user_agent) {
            headers.insert(USER_AGENT, ua);
        }
        headers
    }

    /// Resolve a URL, converting rsshub:// protocol or rsshub.app URLs to configured instance
    pub fn resolve_url(&self, url: &str) -> Result<String> {
        if url.starts_with(&format!("{}://", RSSHUB_SCHEME)) {
            self.convert_rsshub_url(url)
        } else if let Some(path) = url.strip_prefix("https://rsshub.app/") {
            // Auto-convert rsshub.app URLs to configured instance
            self.convert_rsshub_path(path)
        } else if let Some(path) = url.strip_prefix("http://rsshub.app/") {
            self.convert_rsshub_path(path)
        } else {
            // Validate it's a proper URL
            Url::parse(url)?;
            Ok(url.to_string())
        }
    }

    /// Convert a path to the configured RSSHub instance URL
    fn convert_rsshub_path(&self, path: &str) -> Result<String> {
        let base = self.rsshub_base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');

        let mut resolved_url = format!("{}/{}", base, path);

        // Add access key if configured
        if let Some(ref key) = self.rsshub_access_key {
            let separator = if resolved_url.contains('?') { "&" } else { "?" };
            resolved_url = format!("{}{}key={}", resolved_url, separator, key);
        }

        Ok(resolved_url)
    }

    /// Convert rsshub://path to https://rsshub.app/path
    /// If access_key is configured, it will be added as a query parameter
    fn convert_rsshub_url(&self, url: &str) -> Result<String> {
        let path = url
            .strip_prefix(&format!("{}://", RSSHUB_SCHEME))
            .ok_or_else(|| Error::InvalidRsshubUrl(url.to_string()))?;

        let base = self.rsshub_base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');

        let mut resolved_url = format!("{}/{}", base, path);

        // Add access key if configured
        if let Some(ref key) = self.rsshub_access_key {
            let separator = if resolved_url.contains('?') { "&" } else { "?" };
            resolved_url = format!("{}{}key={}", resolved_url, separator, key);
        }

        Ok(resolved_url)
    }

    /// Fetch with retry and exponential backoff
    async fn fetch_with_retry(&self, url: &str) -> Result<(reqwest::StatusCode, HeaderMap, Bytes)> {
        let mut last_error = None;
        let mut delay_ms = INITIAL_RETRY_DELAY_MS;

        for attempt in 0..MAX_RETRIES {
            let user_agent = next_user_agent();
            let headers = Self::build_headers(user_agent);

            tracing::debug!(
                "Fetch attempt {} for {}, User-Agent: {}",
                attempt + 1,
                url,
                user_agent
            );

            match self.client
                .get(url)
                .headers(headers)
                .send()
                .await
            {
                Ok(response) => {
                    let status = response.status();
                    let resp_headers = response.headers().clone();

                    // Check if we should retry (429 Too Many Requests or 503 Service Unavailable)
                    if status == reqwest::StatusCode::TOO_MANY_REQUESTS
                        || status == reqwest::StatusCode::SERVICE_UNAVAILABLE
                    {
                        tracing::warn!(
                            "Received {} for {}, retrying after {}ms...",
                            status,
                            url,
                            delay_ms
                        );
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        delay_ms *= 2; // Exponential backoff
                        last_error = Some(Error::FeedParse(format!("HTTP {} for URL: {}", status, url)));
                        continue;
                    }

                    // For 403, try with different User-Agent before giving up
                    if status == reqwest::StatusCode::FORBIDDEN && attempt < MAX_RETRIES - 1 {
                        tracing::warn!(
                            "Received 403 for {}, trying different User-Agent...",
                            url
                        );
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        delay_ms *= 2;
                        last_error = Some(Error::FeedParse(format!("HTTP 403 Forbidden for URL: {}", url)));
                        continue;
                    }

                    match response.bytes().await {
                        Ok(bytes) => return Ok((status, resp_headers, bytes)),
                        Err(e) => {
                            tracing::warn!("Failed to read response body: {}", e);
                            last_error = Some(Error::Http(e));
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Request failed for {} (attempt {}): {}",
                        url,
                        attempt + 1,
                        e
                    );
                    last_error = Some(Error::Http(e));
                }
            }

            if attempt < MAX_RETRIES - 1 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                delay_ms *= 2;
            }
        }

        Err(last_error.unwrap_or_else(|| Error::FeedParse(format!("Failed to fetch URL after {} retries: {}", MAX_RETRIES, url))))
    }

    /// Fetch and parse a feed from URL
    pub async fn fetch(&self, url: &str, feed_id: Uuid) -> Result<ParsedFeed> {
        let resolved_url = self.resolve_url(url)?;

        tracing::info!("Fetching feed from: {}", resolved_url);

        let (status, resp_headers, content) = self.fetch_with_retry(&resolved_url).await?;

        self.ensure_content_size(content.len(), &resolved_url)?;

        // Check for Cloudflare challenge (403 with specific headers)
        if status == reqwest::StatusCode::FORBIDDEN {
            let is_cloudflare = resp_headers.get("cf-mitigated").is_some()
                || resp_headers
                    .get("server")
                    .map(|v| v.to_str().unwrap_or("").contains("cloudflare"))
                    .unwrap_or(false);

            if is_cloudflare {
                return Err(Error::FeedParse(format!(
                    "Cloudflare protection detected for URL: {}. \
                    The site requires JavaScript verification. \
                    If this is an RSSHub URL, please configure your own RSSHub instance \
                    in config.toml under [rsshub] base_url, or configure a proxy.",
                    resolved_url
                )));
            }

            return Err(Error::FeedParse(format!(
                "HTTP 403 Forbidden for URL: {}. \
                Try configuring an access_key in [rsshub] section or using a proxy.",
                resolved_url
            )));
        }

        if !status.is_success() {
            return Err(Error::FeedParse(format!(
                "HTTP {} for URL: {}",
                status,
                resolved_url
            )));
        }

        // Check if response is a Cloudflare challenge page (HTML with JS challenge)
        if self.is_cloudflare_challenge(&content) {
            return Err(Error::FeedParse(format!(
                "Cloudflare JavaScript challenge detected for URL: {}. \
                The site requires browser verification. \
                If this is an RSSHub URL, please configure your own RSSHub instance \
                in config.toml under [rsshub] base_url, or configure a proxy.",
                resolved_url
            )));
        }

        parse_feed(&content, feed_id)
    }

    /// Check if content is a Cloudflare challenge page
    fn is_cloudflare_challenge(&self, content: &[u8]) -> bool {
        // Check first 2KB for Cloudflare markers
        let check_len = content.len().min(2048);
        let preview = String::from_utf8_lossy(&content[..check_len]);

        // Cloudflare challenge indicators
        preview.contains("Just a moment...")
            || preview.contains("cf-browser-verification")
            || preview.contains("_cf_chl_opt")
            || preview.contains("challenge-platform")
    }

    /// Fetch feed content as raw bytes (for testing URL validity)
    pub async fn fetch_raw(&self, url: &str) -> Result<Vec<u8>> {
        let resolved_url = self.resolve_url(url)?;

        let (status, resp_headers, bytes) = self.fetch_with_retry(&resolved_url).await?;

        self.ensure_content_size(bytes.len(), &resolved_url)?;

        // Check for Cloudflare challenge (403 with specific headers)
        if status == reqwest::StatusCode::FORBIDDEN {
            let is_cloudflare = resp_headers.get("cf-mitigated").is_some()
                || resp_headers
                    .get("server")
                    .map(|v| v.to_str().unwrap_or("").contains("cloudflare"))
                    .unwrap_or(false);

            if is_cloudflare {
                return Err(Error::FeedParse(format!(
                    "Cloudflare protection detected for URL: {}. \
                    The site requires JavaScript verification. \
                    If this is an RSSHub URL, please configure your own RSSHub instance \
                    in config.toml under [rsshub] base_url, or configure a proxy.",
                    resolved_url
                )));
            }

            return Err(Error::FeedParse(format!(
                "HTTP 403 Forbidden for URL: {}. \
                Try configuring an access_key in [rsshub] section or using a proxy.",
                resolved_url
            )));
        }

        if !status.is_success() {
            return Err(Error::FeedParse(format!(
                "HTTP {} for URL: {}",
                status,
                resolved_url
            )));
        }

        // Check if response is a Cloudflare challenge page
        if self.is_cloudflare_challenge(&bytes) {
            return Err(Error::FeedParse(format!(
                "Cloudflare JavaScript challenge detected for URL: {}. \
                The site requires browser verification. \
                If this is an RSSHub URL, please configure your own RSSHub instance \
                in config.toml under [rsshub] base_url, or configure a proxy.",
                resolved_url
            )));
        }

        Ok(bytes.to_vec())
    }

    fn ensure_content_size(&self, size: usize, url: &str) -> Result<()> {
        if size > MAX_FEED_BYTES {
            return Err(Error::FeedParse(format!(
                "Feed too large ({} bytes) for URL: {}",
                size,
                url
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsshub_url_conversion() {
        let config = AppConfig::default();
        let fetcher = FeedFetcher::new(&config).unwrap();

        let result = fetcher.resolve_url("rsshub://hackernews").unwrap();
        assert_eq!(result, "https://rsshub.app/hackernews");

        let result = fetcher.resolve_url("rsshub://twitter/user/test").unwrap();
        assert_eq!(result, "https://rsshub.app/twitter/user/test");
    }

    #[test]
    fn test_regular_url_passthrough() {
        let config = AppConfig::default();
        let fetcher = FeedFetcher::new(&config).unwrap();

        let result = fetcher.resolve_url("https://example.com/feed.xml").unwrap();
        assert_eq!(result, "https://example.com/feed.xml");
    }

    #[test]
    fn test_user_agent_rotation() {
        // Reset counter for deterministic test
        USER_AGENT_INDEX.store(0, Ordering::Relaxed);

        let ua1 = next_user_agent();
        let ua2 = next_user_agent();
        let ua3 = next_user_agent();

        // Should rotate through different user agents
        assert!(ua1.contains("Chrome") && ua1.contains("Macintosh"));
        assert!(ua2.contains("Chrome") && ua2.contains("Windows"));
        assert!(ua3.contains("Firefox") && ua3.contains("Macintosh"));
    }
}
