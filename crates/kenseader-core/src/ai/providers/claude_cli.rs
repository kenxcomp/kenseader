use std::process::Command;

use super::AiProvider;
use crate::{Error, Result};

fn truncate_chars(input: &str, max_chars: usize) -> &str {
    match input.char_indices().nth(max_chars) {
        Some((idx, _)) => &input[..idx],
        None => input,
    }
}

/// Claude CLI provider - uses `claude -p` command
pub struct ClaudeCliProvider;

impl ClaudeCliProvider {
    pub fn new() -> Self {
        Self
    }

    fn run_claude(&self, prompt: &str) -> Result<String> {
        let output = Command::new("claude")
            .arg("-p")
            .arg(prompt)
            .output()
            .map_err(|e| Error::AiProvider(format!("Failed to run claude CLI: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::AiProvider(format!("Claude CLI error: {}", stderr)));
        }

        let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(result)
    }
}

#[async_trait::async_trait]
impl AiProvider for ClaudeCliProvider {
    async fn summarize(&self, content: &str) -> Result<String> {
        let truncated = truncate_chars(content, 4000);

        let prompt = format!(
            "Summarize the following article in 2-3 sentences. Be concise and focus on the key points:\n\n{}",
            truncated
        );

        // Run in blocking context since claude CLI is synchronous
        let prompt_clone = prompt.clone();
        tokio::task::spawn_blocking(move || {
            let provider = ClaudeCliProvider::new();
            provider.run_claude(&prompt_clone)
        })
        .await
        .map_err(|e| Error::AiProvider(format!("Task join error: {}", e)))?
    }

    async fn extract_tags(&self, content: &str) -> Result<Vec<String>> {
        let truncated = truncate_chars(content, 4000);

        let prompt = format!(
            "Extract 3-5 topic tags from the following article. Return only the tags as a comma-separated list, nothing else:\n\n{}",
            truncated
        );

        let prompt_clone = prompt.clone();
        let result = tokio::task::spawn_blocking(move || {
            let provider = ClaudeCliProvider::new();
            provider.run_claude(&prompt_clone)
        })
        .await
        .map_err(|e| Error::AiProvider(format!("Task join error: {}", e)))??;

        let tags: Vec<String> = result
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(tags)
    }

    async fn score_relevance(&self, content: &str, interests: &[String]) -> Result<f64> {
        if interests.is_empty() {
            return Ok(0.5); // Neutral score if no interests defined
        }

        let truncated = truncate_chars(content, 3000);

        let interests_str = interests.join(", ");
        let prompt = format!(
            "Rate how relevant this article is to someone interested in: {}.\n\nArticle:\n{}\n\nRespond with only a number from 0 to 100, where 0 means not relevant at all and 100 means highly relevant.",
            interests_str,
            truncated
        );

        let prompt_clone = prompt.clone();
        let result = tokio::task::spawn_blocking(move || {
            let provider = ClaudeCliProvider::new();
            provider.run_claude(&prompt_clone)
        })
        .await
        .map_err(|e| Error::AiProvider(format!("Task join error: {}", e)))??;

        // Parse the score
        let score: f64 = result
            .trim()
            .parse()
            .unwrap_or(50.0);

        Ok(score / 100.0)
    }
}

impl Default for ClaudeCliProvider {
    fn default() -> Self {
        Self::new()
    }
}
