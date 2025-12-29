mod claude_cli;
mod openai;

pub use claude_cli::ClaudeCliProvider;
pub use openai::OpenAiProvider;

use crate::Result;

/// Trait for AI summarization providers
#[async_trait::async_trait]
pub trait AiProvider: Send + Sync {
    /// Generate a summary for the given content
    async fn summarize(&self, content: &str) -> Result<String>;

    /// Extract tags from content
    async fn extract_tags(&self, content: &str) -> Result<Vec<String>>;

    /// Score article relevance based on user interests
    async fn score_relevance(&self, content: &str, interests: &[String]) -> Result<f64>;
}
