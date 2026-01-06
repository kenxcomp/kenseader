//! IPC Protocol definitions for daemon-client communication
//!
//! Uses JSON-RPC style request/response format over Unix socket.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::feed::{Article, Feed};

/// JSON-RPC style request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: Uuid,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

impl Request {
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            method: method.into(),
            params: serde_json::Value::Null,
        }
    }

    pub fn with_params(mut self, params: serde_json::Value) -> Self {
        self.params = params;
        self
    }
}

/// JSON-RPC style response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl Response {
    pub fn success(id: Uuid, result: serde_json::Value) -> Self {
        Self {
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Uuid, code: i32, message: impl Into<String>) -> Self {
        Self {
            id,
            result: None,
            error: Some(RpcError {
                code,
                message: message.into(),
            }),
        }
    }

    pub fn ok(id: Uuid) -> Self {
        Self::success(id, serde_json::json!({"ok": true}))
    }

    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }
}

/// RPC error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

// Error codes
pub const ERR_PARSE: i32 = -32700;
pub const ERR_INVALID_REQUEST: i32 = -32600;
pub const ERR_METHOD_NOT_FOUND: i32 = -32601;
pub const ERR_INVALID_PARAMS: i32 = -32602;
pub const ERR_INTERNAL: i32 = -32603;
pub const ERR_DAEMON_NOT_RUNNING: i32 = -32000;

// Method names
pub mod methods {
    pub const PING: &str = "ping";
    pub const STATUS: &str = "status";

    // Article methods
    pub const ARTICLE_LIST: &str = "article.list";
    pub const ARTICLE_GET: &str = "article.get";
    pub const ARTICLE_MARK_READ: &str = "article.mark_read";
    pub const ARTICLE_MARK_UNREAD: &str = "article.mark_unread";
    pub const ARTICLE_TOGGLE_SAVED: &str = "article.toggle_saved";
    pub const ARTICLE_SEARCH: &str = "article.search";

    // Feed methods
    pub const FEED_LIST: &str = "feed.list";
    pub const FEED_ADD: &str = "feed.add";
    pub const FEED_DELETE: &str = "feed.delete";
    pub const FEED_REFRESH: &str = "feed.refresh";
}

// Parameter structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleListParams {
    pub feed_id: Option<Uuid>,
    #[serde(default)]
    pub unread_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleIdParams {
    pub id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleSearchParams {
    pub query: String,
    pub feed_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedAddParams {
    pub url: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedIdParams {
    pub id: Option<Uuid>,
}

// Response structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResponse {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub running: bool,
    pub uptime_secs: u64,
    pub scheduler_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleListResponse {
    pub articles: Vec<Article>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleResponse {
    pub article: Option<Article>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedListResponse {
    pub feeds: Vec<Feed>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedAddResponse {
    pub feed: Feed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToggleSavedResponse {
    pub is_saved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshResponse {
    pub new_articles: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub articles: Vec<Article>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = Request::new("ping");
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"method\":\"ping\""));
    }

    #[test]
    fn test_response_success() {
        let id = Uuid::new_v4();
        let resp = Response::ok(id);
        assert!(resp.is_success());
    }

    #[test]
    fn test_response_error() {
        let id = Uuid::new_v4();
        let resp = Response::error(id, ERR_METHOD_NOT_FOUND, "Method not found");
        assert!(!resp.is_success());
        assert_eq!(resp.error.unwrap().code, ERR_METHOD_NOT_FOUND);
    }
}
