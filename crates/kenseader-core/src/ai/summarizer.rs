use std::sync::Arc;

use super::providers::{
    AiProvider, ClaudeApiProvider, ClaudeCliProvider, CliProvider, CliType,
    GeminiApiProvider, OpenAiProvider,
};
pub use super::providers::{ArticleForSummary, BatchSummaryResult};
use crate::config::AppConfig;
use crate::Result;

/// AI Summarizer that wraps the configured provider
pub struct Summarizer {
    provider: Arc<dyn AiProvider>,
    concurrency: usize,
}

impl Summarizer {
    /// Create a new summarizer based on configuration
    pub fn new(config: &AppConfig) -> Result<Self> {
        let language = &config.ai.summary_language;
        let summary_max_tokens = config.ai.max_summary_tokens.max(1);
        let concurrency = config.ai.concurrency.max(1);

        let provider: Arc<dyn AiProvider> = match config.ai.provider.as_str() {
            // API-based providers
            "openai" => {
                let api_key = config.ai.openai_api_key.as_ref()
                    .ok_or_else(|| crate::Error::Config("OpenAI API key not configured".to_string()))?;
                Arc::new(OpenAiProvider::new(api_key, &config.ai.openai_model, language, summary_max_tokens))
            }
            "gemini_api" => {
                let api_key = config.ai.gemini_api_key.as_ref()
                    .ok_or_else(|| crate::Error::Config("Gemini API key not configured".to_string()))?;
                Arc::new(GeminiApiProvider::new(api_key, &config.ai.gemini_model, language, summary_max_tokens))
            }
            "claude_api" => {
                let api_key = config.ai.claude_api_key.as_ref()
                    .ok_or_else(|| crate::Error::Config("Claude API key not configured".to_string()))?;
                Arc::new(ClaudeApiProvider::new(api_key, &config.ai.claude_model, language, summary_max_tokens))
            }
            // CLI-based providers
            "gemini_cli" => {
                Arc::new(CliProvider::new(CliType::Gemini, language))
            }
            "codex_cli" => {
                Arc::new(CliProvider::new(CliType::Codex, language))
            }
            "claude_cli" | _ => {
                Arc::new(ClaudeCliProvider::new(language))
            }
        };

        Ok(Self { provider, concurrency })
    }

    /// Generate a summary for article content
    pub async fn summarize(&self, content: &str) -> Result<String> {
        self.provider.summarize(content).await
    }

    /// Extract tags from article content
    pub async fn extract_tags(&self, content: &str) -> Result<Vec<String>> {
        self.provider.extract_tags(content).await
    }

    /// Score article relevance to user interests
    pub async fn score_relevance(&self, content: &str, interests: &[String]) -> Result<f64> {
        self.provider.score_relevance(content, interests).await
    }

    /// Batch summarize multiple articles in one API call
    pub async fn batch_summarize(&self, articles: Vec<ArticleForSummary>) -> Result<Vec<BatchSummaryResult>> {
        self.provider.batch_summarize(articles).await
    }

    /// Get the character limit for batch processing
    pub fn batch_char_limit(&self) -> usize {
        self.provider.batch_char_limit()
    }

    /// Get minimum content length for summarization
    pub fn min_content_length(&self) -> usize {
        self.provider.min_content_length()
    }

    /// Get max concurrent summarization tasks
    pub fn concurrency(&self) -> usize {
        self.concurrency
    }
}
