use std::sync::Arc;

use super::analyzer::ProfileAnalyzer;
use super::models::TimeWindow;
use crate::ai::Summarizer;
use crate::feed::Article;
use crate::storage::{ArticleRepository, Database};
use crate::Result;

/// Article filter that combines AI scoring with user profile
pub struct ArticleFilter<'a> {
    db: &'a Database,
    summarizer: Option<Arc<Summarizer>>,
    relevance_threshold: f64,
}

impl<'a> ArticleFilter<'a> {
    pub fn new(db: &'a Database, summarizer: Option<Arc<Summarizer>>) -> Self {
        Self {
            db,
            summarizer,
            relevance_threshold: 0.3, // Default threshold
        }
    }

    /// Set the relevance threshold (0.0 - 1.0)
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.relevance_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Filter articles based on combined AI + profile scoring
    /// Returns articles that pass the threshold, marks filtered ones as read
    pub async fn filter_articles(&self, articles: &[Article]) -> Result<Vec<Article>> {
        if articles.is_empty() {
            return Ok(Vec::new());
        }

        // Get user interests from profile
        let analyzer = ProfileAnalyzer::new(self.db);
        let interests = analyzer.get_top_tags(TimeWindow::Last30Days, 10).await?;

        let mut passed = Vec::new();
        let article_repo = ArticleRepository::new(self.db);

        for article in articles {
            let score = self.score_article(article, &interests).await?;

            if score >= self.relevance_threshold {
                passed.push(article.clone());
            } else {
                // Mark filtered article as read (not deleted)
                article_repo.mark_read(article.id).await?;
                tracing::debug!(
                    "Filtered article '{}' with score {:.2}",
                    article.title,
                    score
                );
            }
        }

        tracing::info!(
            "Filtered {} articles, {} passed threshold",
            articles.len(),
            passed.len()
        );

        Ok(passed)
    }

    /// Score a single article based on content and user interests
    async fn score_article(&self, article: &Article, interests: &[String]) -> Result<f64> {
        let content = article.content_text.as_deref()
            .or(article.summary.as_deref())
            .unwrap_or(&article.title);

        // Calculate profile match score
        let profile_score = self.calculate_profile_score(article, interests);

        // Get AI relevance score if summarizer is available
        let ai_score = if let Some(ref summarizer) = self.summarizer {
            match summarizer.score_relevance(content, interests).await {
                Ok(score) => score,
                Err(e) => {
                    tracing::warn!("AI scoring failed: {}", e);
                    1.0 // Pass article through on error
                }
            }
        } else {
            1.0 // No AI configured - pass all articles through
        };

        // Combined score: 40% profile, 60% AI
        let combined = profile_score * 0.4 + ai_score * 0.6;

        Ok(combined)
    }

    /// Calculate profile-based score using tag matching
    fn calculate_profile_score(&self, article: &Article, interests: &[String]) -> f64 {
        if interests.is_empty() || article.tags.is_empty() {
            return 1.0; // No profile data - pass article through
        }

        let matches = article.tags.iter()
            .filter(|tag| interests.iter().any(|i| i.eq_ignore_ascii_case(tag)))
            .count();

        let score = matches as f64 / interests.len().min(article.tags.len()) as f64;
        score.min(1.0)
    }
}
