use std::sync::Arc;

use super::providers::{AiProvider, ClaudeCliProvider, OpenAiProvider};
use crate::config::AppConfig;
use crate::Result;

/// AI Summarizer that wraps the configured provider
pub struct Summarizer {
    provider: Arc<dyn AiProvider>,
}

impl Summarizer {
    /// Create a new summarizer based on configuration
    pub fn new(config: &AppConfig) -> Result<Self> {
        let provider: Arc<dyn AiProvider> = match config.ai.provider.as_str() {
            "openai" => {
                let api_key = config.ai.openai_api_key.as_ref()
                    .ok_or_else(|| crate::Error::Config("OpenAI API key not configured".to_string()))?;
                Arc::new(OpenAiProvider::new(api_key, &config.ai.openai_model))
            }
            "claude_cli" | _ => {
                Arc::new(ClaudeCliProvider::new())
            }
        };

        Ok(Self { provider })
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
}
