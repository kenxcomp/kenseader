use std::process::Command;

use super::{AiProvider, ArticleForScoring, ArticleForSummary, ArticleStyleResult, BatchScoreResult, BatchSummaryResult};
use crate::{Error, Result};

fn truncate_chars(input: &str, max_chars: usize) -> &str {
    match input.char_indices().nth(max_chars) {
        Some((idx, _)) => &input[..idx],
        None => input,
    }
}

/// CLI provider type
#[derive(Debug, Clone, Copy)]
pub enum CliType {
    Claude,
    Gemini,
    Codex,
}

impl CliType {
    fn command(&self) -> &'static str {
        match self {
            CliType::Claude => "claude",
            CliType::Gemini => "gemini",
            CliType::Codex => "codex",
        }
    }

    fn base_args(&self) -> Vec<&'static str> {
        match self {
            CliType::Claude => vec!["-p", "--tools", ""],
            CliType::Gemini => vec!["-p"],
            CliType::Codex => vec!["exec"],  // Codex uses exec subcommand
        }
    }

    fn uses_stdin(&self) -> bool {
        match self {
            CliType::Claude | CliType::Gemini => true,
            CliType::Codex => false,  // Codex takes prompt as command line argument
        }
    }
}

/// Generic CLI provider for Claude, Gemini, Codex
pub struct CliProvider {
    cli_type: CliType,
    language: String,
    summary_max_length: usize,
    min_content_length: usize,
}

impl CliProvider {
    pub fn new(cli_type: CliType, language: &str, summary_max_length: usize, min_content_length: usize) -> Self {
        Self {
            cli_type,
            language: language.to_string(),
            summary_max_length,
            min_content_length,
        }
    }

    fn run_cli(&self, prompt: &str) -> Result<String> {
        use std::io::Write;
        use std::process::Stdio;

        let mut cmd = Command::new(self.cli_type.command());
        for arg in self.cli_type.base_args() {
            cmd.arg(arg);
        }

        // Codex: prompt as command line argument
        if !self.cli_type.uses_stdin() {
            cmd.arg(prompt);
        }

        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::AiProvider(format!(
                "Failed to spawn {} CLI: {}",
                self.cli_type.command(),
                e
            )))?;

        // Claude/Gemini: prompt via stdin
        if self.cli_type.uses_stdin() {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(prompt.as_bytes())
                    .map_err(|e| Error::AiProvider(format!(
                        "Failed to write to {} stdin: {}",
                        self.cli_type.command(),
                        e
                    )))?;
            }
        }

        let output = child.wait_with_output()
            .map_err(|e| Error::AiProvider(format!(
                "Failed to wait for {} CLI: {}",
                self.cli_type.command(),
                e
            )))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::AiProvider(format!(
                "{} CLI error: {}",
                self.cli_type.command(),
                stderr
            )));
        }

        let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(result)
    }
}

#[async_trait::async_trait]
impl AiProvider for CliProvider {
    fn language(&self) -> &str {
        &self.language
    }

    async fn summarize(&self, content: &str) -> Result<String> {
        let trimmed = content.trim();
        if trimmed.len() < self.min_content_length {
            return Err(Error::AiProvider(format!(
                "Content too short to summarize ({} chars, minimum {})",
                trimmed.len(),
                self.min_content_length
            )));
        }
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            if trimmed.lines().count() <= 2 {
                return Err(Error::AiProvider("Content appears to be just a URL".to_string()));
            }
        }

        let truncated = truncate_chars(content, 4000);
        let language = &self.language;
        let max_len = self.summary_max_length;

        let prompt = format!(
            "Below is the full text of an article. Summarize it in 2-3 sentences (max {max_len} characters) in {language}. \
Do NOT try to fetch any URLs. Use ONLY the text provided below.\n\n\
---BEGIN ARTICLE TEXT---\n{truncated}\n---END ARTICLE TEXT---\n\n\
Summary (in {language}, max {max_len} chars):"
        );

        let prompt_clone = prompt.clone();
        let cli_type = self.cli_type;
        let lang = self.language.clone();
        let min_len = self.min_content_length;

        tokio::task::spawn_blocking(move || {
            let provider = CliProvider::new(cli_type, &lang, max_len, min_len);
            provider.run_cli(&prompt_clone)
        })
        .await
        .map_err(|e| Error::AiProvider(format!("Task join error: {}", e)))?
    }

    async fn extract_tags(&self, content: &str) -> Result<Vec<String>> {
        let trimmed = content.trim();
        if trimmed.len() < 50 {
            return Ok(Vec::new());
        }

        let truncated = truncate_chars(content, 4000);

        let prompt = format!(
            "Extract 3-5 topic tags from the article text below. \
Return ONLY the tags as a comma-separated list, nothing else. \
Do NOT try to fetch any URLs.\n\n\
---BEGIN ARTICLE TEXT---\n{truncated}\n---END ARTICLE TEXT---\n\n\
Tags:"
        );

        let prompt_clone = prompt.clone();
        let cli_type = self.cli_type;
        let lang = self.language.clone();
        let max_len = self.summary_max_length;
        let min_len = self.min_content_length;

        let result = tokio::task::spawn_blocking(move || {
            let provider = CliProvider::new(cli_type, &lang, max_len, min_len);
            provider.run_cli(&prompt_clone)
        })
        .await
        .map_err(|e| Error::AiProvider(format!("Task join error: {}", e)))??;

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
Respond with only a number from 0 to 100, where 0 means not relevant at all and 100 means highly relevant.",
            interests_str, truncated
        );

        let prompt_clone = prompt.clone();
        let cli_type = self.cli_type;
        let lang = self.language.clone();
        let max_len = self.summary_max_length;
        let min_len = self.min_content_length;

        let result = tokio::task::spawn_blocking(move || {
            let provider = CliProvider::new(cli_type, &lang, max_len, min_len);
            provider.run_cli(&prompt_clone)
        })
        .await
        .map_err(|e| Error::AiProvider(format!("Task join error: {}", e)))??;

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
        let max_len = self.summary_max_length;
        let mut prompt = format!(
            "Below are multiple articles. For EACH article, provide a 2-3 sentence summary (max {max_len} characters) in {language}.\n\
Do NOT fetch any URLs. Use ONLY the text provided.\n\
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
            "Now provide summaries in {language} (max {max_len} chars each) using the format [ARTICLE_ID]: summary\n"
        ));

        let prompt_clone = prompt;
        let cli_type = self.cli_type;
        let lang = self.language.clone();
        let min_len_for_spawn = self.min_content_length;

        let result = tokio::task::spawn_blocking(move || {
            let provider = CliProvider::new(cli_type, &lang, max_len, min_len_for_spawn);
            provider.run_cli(&prompt_clone)
        })
        .await
        .map_err(|e| Error::AiProvider(format!("Task join error: {}", e)))??;

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

        let prompt_clone = prompt;
        let cli_type = self.cli_type;
        let lang = self.language.clone();
        let max_len = self.summary_max_length;
        let min_len = self.min_content_length;

        let result = tokio::task::spawn_blocking(move || {
            let provider = CliProvider::new(cli_type, &lang, max_len, min_len);
            provider.run_cli(&prompt_clone)
        })
        .await
        .map_err(|e| Error::AiProvider(format!("Task join error: {}", e)))??;

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
        self.min_content_length
    }

    async fn classify_style(&self, content: &str) -> Result<ArticleStyleResult> {
        let truncated = truncate_chars(content, 2000);

        let prompt = format!(
            "Classify this article's style. Respond with ONLY valid JSON (no markdown, no code blocks):\n\
            {{\"style_type\": \"tutorial|news|opinion|analysis|review\", \"tone\": \"formal|casual|technical|humorous\", \"length_category\": \"short|medium|long\"}}\n\n\
            Choose the most appropriate value for each field based on the article content.\n\n\
            Article:\n{truncated}"
        );

        let prompt_clone = prompt;
        let cli_type = self.cli_type;
        let lang = self.language.clone();
        let max_len = self.summary_max_length;
        let min_len = self.min_content_length;

        let result = tokio::task::spawn_blocking(move || {
            let provider = CliProvider::new(cli_type, &lang, max_len, min_len);
            provider.run_cli(&prompt_clone)
        })
        .await
        .map_err(|e| Error::AiProvider(format!("Task join error: {}", e)))??;

        // Parse JSON response
        let cleaned = result.trim().trim_matches(|c| c == '`' || c == '\n');
        Ok(serde_json::from_str(cleaned).unwrap_or_else(|_| ArticleStyleResult::default()))
    }
}
