mod claude_api;
mod claude_cli;
mod cli_base;
mod gemini_api;
mod openai;

pub use claude_api::ClaudeApiProvider;
pub use claude_cli::ClaudeCliProvider;
pub use cli_base::{CliProvider, CliType};
pub use gemini_api::GeminiApiProvider;
pub use openai::OpenAiProvider;

use crate::Result;

/// Article info for batch summarization
#[derive(Debug, Clone)]
pub struct ArticleForSummary {
    pub id: String,
    pub title: String,
    pub content: String,
}

/// Result of batch summarization
#[derive(Debug, Clone)]
pub struct BatchSummaryResult {
    pub id: String,
    pub summary: Option<String>,
    pub error: Option<String>,
}

/// Article info for batch scoring
#[derive(Debug, Clone)]
pub struct ArticleForScoring {
    pub id: String,
    pub content: String, // title + summary or title + content
}

/// Result of batch scoring
#[derive(Debug, Clone)]
pub struct BatchScoreResult {
    pub id: String,
    pub score: Option<f64>,
    pub error: Option<String>,
}

/// Result of article style classification
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArticleStyleResult {
    /// Article style type: tutorial, news, opinion, analysis, review
    pub style_type: String,
    /// Article tone: formal, casual, technical, humorous
    pub tone: String,
    /// Article length category: short, medium, long
    pub length_category: String,
}

impl Default for ArticleStyleResult {
    fn default() -> Self {
        Self {
            style_type: "news".to_string(),
            tone: "formal".to_string(),
            length_category: "medium".to_string(),
        }
    }
}

/// Trait for AI summarization providers
#[async_trait::async_trait]
pub trait AiProvider: Send + Sync {
    /// Get the configured summary language
    fn language(&self) -> &str {
        "English"
    }

    /// Generate a summary for the given content
    async fn summarize(&self, content: &str) -> Result<String>;

    /// Extract tags from content
    async fn extract_tags(&self, content: &str) -> Result<Vec<String>>;

    /// Score article relevance based on user interests
    async fn score_relevance(&self, content: &str, interests: &[String]) -> Result<f64>;

    /// Batch summarize multiple articles in one API call
    /// Returns a vector of results matching the input order
    async fn batch_summarize(&self, articles: Vec<ArticleForSummary>) -> Result<Vec<BatchSummaryResult>>;

    /// Batch score multiple articles for relevance in one API call
    /// Returns a vector of results matching the input order
    async fn batch_score_relevance(
        &self,
        articles: Vec<ArticleForScoring>,
        interests: &[String],
    ) -> Result<Vec<BatchScoreResult>>;

    /// Get the maximum token/character limit for batch processing
    fn batch_char_limit(&self) -> usize {
        80000 // ~20K tokens, conservative default
    }

    /// Get minimum content length for summarization
    fn min_content_length(&self) -> usize {
        1000
    }

    /// Classify article style, tone, and length category
    async fn classify_style(&self, content: &str) -> Result<ArticleStyleResult>;
}
