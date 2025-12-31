use async_openai::{
    types::{ChatCompletionRequestMessage, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs},
    Client,
};

use super::AiProvider;
use crate::{Error, Result};

fn truncate_chars(input: &str, max_chars: usize) -> &str {
    match input.char_indices().nth(max_chars) {
        Some((idx, _)) => &input[..idx],
        None => input,
    }
}

/// OpenAI API provider
pub struct OpenAiProvider {
    client: Client<async_openai::config::OpenAIConfig>,
    model: String,
}

impl OpenAiProvider {
    pub fn new(api_key: &str, model: &str) -> Self {
        let config = async_openai::config::OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(config);

        Self {
            client,
            model: model.to_string(),
        }
    }

    async fn chat(&self, prompt: &str) -> Result<String> {
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(vec![ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(prompt)
                    .build()
                    .map_err(|e| Error::AiProvider(e.to_string()))?,
            )])
            .max_tokens(200u32)
            .build()
            .map_err(|e| Error::AiProvider(e.to_string()))?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| Error::AiProvider(e.to_string()))?;

        let content = response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(content)
    }
}

#[async_trait::async_trait]
impl AiProvider for OpenAiProvider {
    async fn summarize(&self, content: &str) -> Result<String> {
        let truncated = truncate_chars(content, 4000);

        let prompt = format!(
            "Summarize the following article in 2-3 sentences. Be concise and focus on the key points:\n\n{}",
            truncated
        );

        self.chat(&prompt).await
    }

    async fn extract_tags(&self, content: &str) -> Result<Vec<String>> {
        let truncated = truncate_chars(content, 4000);

        let prompt = format!(
            "Extract 3-5 topic tags from the following article. Return only the tags as a comma-separated list, nothing else:\n\n{}",
            truncated
        );

        let result = self.chat(&prompt).await?;

        let tags: Vec<String> = result
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(tags)
    }

    async fn score_relevance(&self, content: &str, interests: &[String]) -> Result<f64> {
        if interests.is_empty() {
            return Ok(0.5);
        }

        let truncated = truncate_chars(content, 3000);

        let interests_str = interests.join(", ");
        let prompt = format!(
            "Rate how relevant this article is to someone interested in: {}.\n\nArticle:\n{}\n\nRespond with only a number from 0 to 100.",
            interests_str,
            truncated
        );

        let result = self.chat(&prompt).await?;

        let score: f64 = result.trim().parse().unwrap_or(50.0);
        Ok(score / 100.0)
    }
}
