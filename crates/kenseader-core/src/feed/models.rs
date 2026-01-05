use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents an RSS/Atom feed subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feed {
    pub id: Uuid,
    pub url: String,
    pub local_name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub site_url: Option<String>,
    pub icon_url: Option<String>,
    pub last_fetched_at: Option<DateTime<Utc>>,
    pub fetch_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Computed field (not stored in DB)
    #[serde(default)]
    pub unread_count: u32,
}

/// Data required to create a new feed
#[derive(Debug, Clone)]
pub struct NewFeed {
    pub url: String,
    pub local_name: String,
}

/// Represents an article from a feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    pub id: Uuid,
    pub feed_id: Uuid,
    pub guid: String,
    pub url: Option<String>,
    pub title: String,
    pub author: Option<String>,
    pub content: Option<String>,
    pub content_text: Option<String>,
    pub summary: Option<String>,
    pub summary_generated_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub fetched_at: DateTime<Utc>,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub is_saved: bool,
    pub created_at: DateTime<Utc>,
    pub image_url: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Data required to create a new article
#[derive(Debug, Clone)]
pub struct NewArticle {
    pub feed_id: Uuid,
    pub guid: String,
    pub url: Option<String>,
    pub title: String,
    pub author: Option<String>,
    pub content: Option<String>,
    pub content_text: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub image_url: Option<String>,
}

impl Article {
    /// Check if the article has been summarized
    pub fn is_summarized(&self) -> bool {
        self.summary.is_some()
    }

    /// Get a preview of the content (first N characters)
    pub fn content_preview(&self, max_len: usize) -> String {
        let text = self.content_text.as_deref()
            .or(self.summary.as_deref())
            .unwrap_or("");

        if max_len == 0 {
            return String::new();
        }

        if text.len() <= max_len {
            text.to_string()
        } else {
            let mut end = 0;
            for (idx, ch) in text.char_indices() {
                let next = idx + ch.len_utf8();
                if next > max_len {
                    break;
                }
                end = next;
            }
            format!("{}...", &text[..end])
        }
    }
}

impl Feed {
    /// Check if the feed has a fetch error
    pub fn has_error(&self) -> bool {
        self.fetch_error.is_some()
    }
}
