use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::{AiProvider, ArticleForScoring, ArticleForSummary, ArticleStyleResult, BatchScoreResult, BatchSummaryResult};
use crate::{Error, Result};

fn truncate_chars(input: &str, max_chars: usize) -> &str {
    match input.char_indices().nth(max_chars) {
        Some((idx, _)) => &input[..idx],
        None => input,
    }
}

const AI_REQUEST_TIMEOUT_SECS: u64 = 30;

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
    temperature: f32,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    error: Option<GeminiError>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContentResponse,
}

#[derive(Deserialize)]
struct GeminiContentResponse {
    parts: Vec<GeminiPartResponse>,
}

#[derive(Deserialize)]
struct GeminiPartResponse {
    text: String,
}

#[derive(Deserialize)]
struct GeminiError {
    message: String,
}

/// Gemini API provider
pub struct GeminiApiProvider {
    client: Client,
    api_key: String,
    model: String,
    language: String,
    summary_max_tokens: u32,
}

impl GeminiApiProvider {
    pub fn new(api_key: &str, model: &str, language: &str, summary_max_tokens: u32) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(AI_REQUEST_TIMEOUT_SECS))
            .build()
            .expect("Failed to build Gemini HTTP client");

        Self {
            client,
            api_key: api_key.to_string(),
            model: model.to_string(),
            language: language.to_string(),
            summary_max_tokens,
        }
    }

    async fn chat(&self, prompt: &str, max_tokens: u32) -> Result<String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let request = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: GenerationConfig {
                max_output_tokens: max_tokens,
                temperature: 0.7,
            },
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::AiProvider(format!("Gemini API request failed: {}", e)))?;

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .map_err(|e| Error::AiProvider(format!("Failed to parse Gemini response: {}", e)))?;

        if let Some(error) = gemini_response.error {
            return Err(Error::AiProvider(format!("Gemini API error: {}", error.message)));
        }

        let content = gemini_response
            .candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.content.parts.into_iter().next())
            .map(|p| p.text)
            .unwrap_or_default();

        Ok(content)
    }
}

#[async_trait::async_trait]
impl AiProvider for GeminiApiProvider {
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
            "Summarize the following article in 2-3 sentences in {language}. \
Be concise and focus on the key points:\n\n{truncated}"
        );

        self.chat(&prompt, self.summary_max_tokens).await
    }

    async fn extract_tags(&self, content: &str) -> Result<Vec<String>> {
        let trimmed = content.trim();
        if trimmed.len() < 50 {
            return Ok(Vec::new());
        }

        let truncated = truncate_chars(content, 4000);

        let prompt = format!(
            "Extract 3-5 topic tags from the following article. \
Return only the tags as a comma-separated list, nothing else:\n\n{truncated}"
        );

        let result = self.chat(&prompt, 100).await?;

        let tags: Vec<String> = result
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty() && s.len() < 50)
            .take(5)
            .collect();

        Ok(tags)
    }

    async fn score_relevance(&self, content: &str, interests: &[String]) -> Result<f64> {
        if interests.is_empty() {
            return Ok(1.0); // No user profile yet - pass article through
        }

        let truncated = truncate_chars(content, 3000);
        let interests_str = interests.join(", ");

        let prompt = format!(
            "Rate how relevant this article is to someone interested in: {}.\n\n\
Article:\n{}\n\n\
Respond with only a number from 0 to 100.",
            interests_str, truncated
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

        let result = self.chat(&prompt, 2000).await?;

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
            // No user profile yet - pass all articles through (score 1.0)
            tracing::info!("No user interests found, passing all {} articles through", articles.len());
            return Ok(articles
                .into_iter()
                .map(|a| BatchScoreResult {
                    id: a.id,
                    score: Some(1.0),
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
        200000 // ~100K tokens (conservative estimate: 2 chars/token for mixed content)
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
