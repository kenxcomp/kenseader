use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{AiProvider, ArticleForSummary, BatchSummaryResult};
use crate::{Error, Result};

fn truncate_chars(input: &str, max_chars: usize) -> &str {
    match input.char_indices().nth(max_chars) {
        Some((idx, _)) => &input[..idx],
        None => input,
    }
}

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
}

impl GeminiApiProvider {
    pub fn new(api_key: &str, model: &str, language: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            language: language.to_string(),
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

        self.chat(&prompt, 200).await
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
            return Ok(0.5);
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

    fn batch_char_limit(&self) -> usize {
        100000 // ~25K tokens for Gemini
    }

    fn min_content_length(&self) -> usize {
        1000
    }
}
