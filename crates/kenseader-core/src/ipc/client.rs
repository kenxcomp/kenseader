//! IPC Client for connecting to daemon
//!
//! Provides a type-safe interface for communicating with the daemon.

use std::path::PathBuf;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use uuid::Uuid;

use super::protocol::*;
use crate::feed::{Article, Feed};
use crate::{Error, Result};

/// Client for communicating with the daemon
#[derive(Clone)]
pub struct DaemonClient {
    socket_path: PathBuf,
}

impl DaemonClient {
    /// Create a new daemon client
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    /// Check if daemon is running by sending a ping
    pub async fn ping(&self) -> Result<bool> {
        match self.call(methods::PING, serde_json::Value::Null).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Get daemon status
    pub async fn status(&self) -> Result<StatusResponse> {
        let result = self.call(methods::STATUS, serde_json::Value::Null).await?;
        Ok(serde_json::from_value(result)?)
    }

    /// List articles
    pub async fn list_articles(
        &self,
        feed_id: Option<Uuid>,
        unread_only: bool,
    ) -> Result<Vec<Article>> {
        let params = serde_json::json!({
            "feed_id": feed_id,
            "unread_only": unread_only
        });
        let result = self.call(methods::ARTICLE_LIST, params).await?;
        let response: ArticleListResponse = serde_json::from_value(result)?;
        Ok(response.articles)
    }

    /// Get a single article by ID
    pub async fn get_article(&self, id: Uuid) -> Result<Option<Article>> {
        let params = serde_json::json!({ "id": id });
        let result = self.call(methods::ARTICLE_GET, params).await?;
        let response: ArticleResponse = serde_json::from_value(result)?;
        Ok(response.article)
    }

    /// Mark an article as read
    pub async fn mark_read(&self, id: Uuid) -> Result<()> {
        let params = serde_json::json!({ "id": id });
        self.call(methods::ARTICLE_MARK_READ, params).await?;
        Ok(())
    }

    /// Mark an article as unread
    pub async fn mark_unread(&self, id: Uuid) -> Result<()> {
        let params = serde_json::json!({ "id": id });
        self.call(methods::ARTICLE_MARK_UNREAD, params).await?;
        Ok(())
    }

    /// Toggle article saved status
    pub async fn toggle_saved(&self, id: Uuid) -> Result<bool> {
        let params = serde_json::json!({ "id": id });
        let result = self.call(methods::ARTICLE_TOGGLE_SAVED, params).await?;
        let response: ToggleSavedResponse = serde_json::from_value(result)?;
        Ok(response.is_saved)
    }

    /// Search articles
    pub async fn search(&self, query: &str, feed_id: Option<Uuid>) -> Result<Vec<Article>> {
        let params = serde_json::json!({
            "query": query,
            "feed_id": feed_id
        });
        let result = self.call(methods::ARTICLE_SEARCH, params).await?;
        let response: SearchResponse = serde_json::from_value(result)?;
        Ok(response.articles)
    }

    /// List all feeds
    pub async fn list_feeds(&self) -> Result<Vec<Feed>> {
        let result = self.call(methods::FEED_LIST, serde_json::Value::Null).await?;
        let response: FeedListResponse = serde_json::from_value(result)?;
        Ok(response.feeds)
    }

    /// Add a new feed
    pub async fn add_feed(&self, url: &str, name: &str) -> Result<Feed> {
        let params = serde_json::json!({
            "url": url,
            "name": name
        });
        let result = self.call(methods::FEED_ADD, params).await?;
        let response: FeedAddResponse = serde_json::from_value(result)?;
        Ok(response.feed)
    }

    /// Delete a feed
    pub async fn delete_feed(&self, id: Uuid) -> Result<bool> {
        let params = serde_json::json!({ "id": id });
        let result = self.call(methods::FEED_DELETE, params).await?;
        let deleted: bool = serde_json::from_value(
            result.get("deleted").cloned().unwrap_or(serde_json::Value::Bool(false)),
        )?;
        Ok(deleted)
    }

    /// Refresh feeds
    pub async fn refresh(&self, feed_id: Option<Uuid>) -> Result<u32> {
        let params = serde_json::json!({ "id": feed_id });
        let result = self.call(methods::FEED_REFRESH, params).await?;
        let response: RefreshResponse = serde_json::from_value(result)?;
        Ok(response.new_articles)
    }

    /// Send a request and receive a response
    async fn call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let stream = UnixStream::connect(&self.socket_path).await.map_err(|e| {
            Error::Other(format!(
                "Failed to connect to daemon at {}: {}. Is the daemon running?",
                self.socket_path.display(),
                e
            ))
        })?;

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Build request
        let request = Request::new(method).with_params(params);
        let request_json = serde_json::to_string(&request)?;

        // Send request
        writer.write_all(request_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        // Read response
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await?;

        let response: Response = serde_json::from_str(&response_line)?;

        if let Some(error) = response.error {
            return Err(Error::Other(format!(
                "RPC error {}: {}",
                error.code, error.message
            )));
        }

        response.result.ok_or_else(|| Error::Other("Empty response".to_string()))
    }
}

/// Check if daemon is reachable
pub async fn is_daemon_running(socket_path: &std::path::Path) -> bool {
    let client = DaemonClient::new(socket_path.to_path_buf());
    client.ping().await.unwrap_or(false)
}
