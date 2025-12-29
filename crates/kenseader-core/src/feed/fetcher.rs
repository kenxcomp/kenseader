use std::time::Duration;
use reqwest::Client;
use url::Url;
use uuid::Uuid;

use super::parser::{parse_feed, ParsedFeed};
use crate::config::AppConfig;
use crate::{Error, Result};

const RSSHUB_SCHEME: &str = "rsshub";

/// Feed fetcher with HTTP client and RSSHub support
pub struct FeedFetcher {
    client: Client,
    rsshub_base_url: String,
}

impl FeedFetcher {
    /// Create a new feed fetcher with configuration
    pub fn new(config: &AppConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.sync.request_timeout_secs))
            .user_agent("Kenseader/0.1.0")
            .build()
            .map_err(|e| Error::Http(e))?;

        Ok(Self {
            client,
            rsshub_base_url: config.rsshub.base_url.clone(),
        })
    }

    /// Resolve a URL, converting rsshub:// protocol if necessary
    pub fn resolve_url(&self, url: &str) -> Result<String> {
        if url.starts_with(&format!("{}://", RSSHUB_SCHEME)) {
            self.convert_rsshub_url(url)
        } else {
            // Validate it's a proper URL
            Url::parse(url)?;
            Ok(url.to_string())
        }
    }

    /// Convert rsshub://path to https://rsshub.app/path
    fn convert_rsshub_url(&self, url: &str) -> Result<String> {
        let path = url
            .strip_prefix(&format!("{}://", RSSHUB_SCHEME))
            .ok_or_else(|| Error::InvalidRsshubUrl(url.to_string()))?;

        let base = self.rsshub_base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');

        Ok(format!("{}/{}", base, path))
    }

    /// Fetch and parse a feed from URL
    pub async fn fetch(&self, url: &str, feed_id: Uuid) -> Result<ParsedFeed> {
        let resolved_url = self.resolve_url(url)?;

        tracing::info!("Fetching feed from: {}", resolved_url);

        let response = self.client
            .get(&resolved_url)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(Error::FeedParse(format!(
                "HTTP {} for URL: {}",
                status,
                resolved_url
            )));
        }

        let content = response.bytes().await?;
        parse_feed(&content, feed_id)
    }

    /// Fetch feed content as raw bytes (for testing URL validity)
    pub async fn fetch_raw(&self, url: &str) -> Result<Vec<u8>> {
        let resolved_url = self.resolve_url(url)?;

        let response = self.client
            .get(&resolved_url)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(Error::FeedParse(format!(
                "HTTP {} for URL: {}",
                status,
                resolved_url
            )));
        }

        Ok(response.bytes().await?.to_vec())
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
}
