use async_openai::{
    types::{ChatCompletionRequestMessage, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs},
    Client,
};

use super::{AiProvider, ArticleForScoring, ArticleForSummary, ArticleStyleResult, BatchScoreResult, BatchSummaryResult};
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
    language: String,
    summary_max_tokens: u32,
}

impl OpenAiProvider {
    pub fn new(api_key: &str, model: &str, language: &str, summary_max_tokens: u32) -> Self {
        let config = async_openai::config::OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(config);

        Self {
            client,
            model: model.to_string(),
            language: language.to_string(),
            summary_max_tokens,
        }
    }

    async fn chat(&self, prompt: &str, max_tokens: u32) -> Result<String> {
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(vec![ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(prompt)
                    .build()
                    .map_err(|e| Error::AiProvider(e.to_string()))?,
            )])
            .max_tokens(max_tokens)
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
    fn language(&self) -> &str {
        &self.language
    }

    async fn summarize(&self, content: &str) -> Result<String> {
        let trimmed = content.trim();
        if trimmed.len() < 1000 {
            return Err(Error::AiProvider(format!(
                "Content too short to summarize ({} chars, minimum 1000)",
                trimmed.len()
            )));
        }

        let truncated = truncate_chars(content, 4000);
        let language = &self.language;

        let prompt = format!(
            "Summarize the following article in 2-3 sentences in {language}. Be concise and focus on the key points:\n\n{truncated}"
        );

        self.chat(&prompt, self.summary_max_tokens).await
    }

    async fn extract_tags(&self, content: &str) -> Result<Vec<String>> {
        let truncated = truncate_chars(content, 4000);

        let prompt = format!(
            "Extract 3-5 topic tags from the following article. Return only the tags as a comma-separated list, nothing else:\n\n{}",
            truncated
        );

        let result = self.chat(&prompt, 100).await?;

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

        let result = self.chat(&prompt, 10).await?;

        let score: f64 = result.trim().parse().unwrap_or(50.0);
        Ok(score / 100.0)
    }

    async fn batch_summarize(&self, articles: Vec<ArticleForSummary>) -> Result<Vec<BatchSummaryResult>> {
        if articles.is_empty() {
            return Ok(Vec::new());
        }

        let min_len = self.min_content_length();
        let valid_articles: Vec<_> = articles
            .iter()
            .filter(|a| a.content.trim().len() >= min_len)
            .collect();

        if valid_articles.is_empty() {
            return Ok(articles
                .into_iter()
                .map(|a| BatchSummaryResult {
                    id: a.id,
                    summary: None,
                    error: Some(format!("Content too short (minimum {} chars)", min_len)),
                })
                .collect());
        }

        // Build batch prompt
        let language = &self.language;
        let mut prompt = format!(
            "Below are multiple articles. For EACH article, provide a 2-3 sentence summary in {language}.\n\
            Format your response EXACTLY as follows, with each summary on its own line:\n\
            [ARTICLE_ID]: summary text here\n\n"
        );

        for article in &valid_articles {
            let truncated = truncate_chars(&article.content, 3000);
            prompt.push_str(&format!(
                "---ARTICLE [{}]: {}---\n{}\n\n",
                article.id, article.title, truncated
            ));
        }

        prompt.push_str(&format!(
            "Now provide summaries in {language} using the format [ARTICLE_ID]: summary\n"
        ));

        let result = self.chat(&prompt, 200).await?;

        // Parse results
        let mut summaries: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for line in result.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                if let Some(end_bracket) = line.find("]:") {
                    let id = &line[1..end_bracket];
                    let summary = line[end_bracket + 2..].trim();
                    if !summary.is_empty() {
                        summaries.insert(id.to_string(), summary.to_string());
                    }
                }
            }
        }

        Ok(articles
            .into_iter()
            .map(|a| {
                if a.content.trim().len() < min_len {
                    BatchSummaryResult {
                        id: a.id,
                        summary: None,
                        error: Some(format!("Content too short (minimum {} chars)", min_len)),
                    }
                } else if let Some(summary) = summaries.get(&a.id) {
                    BatchSummaryResult {
                        id: a.id,
                        summary: Some(summary.clone()),
                        error: None,
                    }
                } else {
                    BatchSummaryResult {
                        id: a.id,
                        summary: None,
                        error: Some("Summary not found in response".to_string()),
                    }
                }
            })
            .collect())
    }

    async fn batch_score_relevance(
        &self,
        articles: Vec<ArticleForScoring>,
        interests: &[String],
    ) -> Result<Vec<BatchScoreResult>> {
        if articles.is_empty() {
            return Ok(Vec::new());
        }

        if interests.is_empty() {
            return Ok(articles
                .into_iter()
                .map(|a| BatchScoreResult {
                    id: a.id,
                    score: Some(0.5),
                    error: None,
                })
                .collect());
        }

        let interests_str = interests.join(", ");
        let mut prompt = format!(
            "Rate how relevant each article is to someone interested in: {interests_str}.\n\
            For EACH article, respond with a score from 0 to 100.\n\
            Format your response EXACTLY as follows, one per line:\n\
            [ARTICLE_ID]: score\n\n"
        );

        for article in &articles {
            let truncated = truncate_chars(&article.content, 1000);
            prompt.push_str(&format!(
                "---ARTICLE [{}]---\n{}\n\n",
                article.id, truncated
            ));
        }

        prompt.push_str("Now provide scores using the format [ARTICLE_ID]: score\n");

        let result = self.chat(&prompt, 200).await?;

        let mut scores: std::collections::HashMap<String, f64> = std::collections::HashMap::new();

        for line in result.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                if let Some(end_bracket) = line.find("]:") {
                    let id = &line[1..end_bracket];
                    let score_str = line[end_bracket + 2..].trim();
                    if let Ok(score) = score_str.parse::<f64>() {
                        scores.insert(id.to_string(), score / 100.0);
                    }
                }
            }
        }

        Ok(articles
            .into_iter()
            .map(|a| {
                if let Some(&score) = scores.get(&a.id) {
                    BatchScoreResult {
                        id: a.id,
                        score: Some(score),
                        error: None,
                    }
                } else {
                    BatchScoreResult {
                        id: a.id,
                        score: None,
                        error: Some("Score not found in response".to_string()),
                    }
                }
            })
            .collect())
    }

    fn batch_char_limit(&self) -> usize {
        100000 // ~25K tokens for GPT-4
    }

    fn min_content_length(&self) -> usize {
        1000
    }

    async fn classify_style(&self, content: &str) -> Result<ArticleStyleResult> {
        let truncated = truncate_chars(content, 2000);

        let prompt = format!(
            "Classify this article's style. Respond with ONLY valid JSON (no markdown, no code blocks):\n\
            {{\"style_type\": \"tutorial|news|opinion|analysis|review\", \"tone\": \"formal|casual|technical|humorous\", \"length_category\": \"short|medium|long\"}}\n\n\
            Choose the most appropriate value for each field based on the article content.\n\n\
            Article:\n{truncated}"
        );

        let result = self.chat(&prompt, 100).await?;

        // Parse JSON response
        let cleaned = result.trim().trim_matches(|c| c == '`' || c == '\n');
        Ok(serde_json::from_str(cleaned).unwrap_or_else(|_| ArticleStyleResult::default()))
    }
}
