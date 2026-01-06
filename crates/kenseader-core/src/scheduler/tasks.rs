use std::sync::Arc;

use uuid::Uuid;

use crate::ai::{ArticleForScoring, ArticleForSummary, Summarizer};
use crate::config::AppConfig;
use crate::feed::FeedFetcher;
use crate::profile::{ProfileAnalyzer, TimeWindow};
use crate::storage::{ArticleRepository, Database, FeedRepository};
use crate::Result;

/// Refresh all feeds and fetch new articles
pub async fn refresh_all_feeds(db: &Database, config: &AppConfig) -> Result<u32> {
    let fetcher = FeedFetcher::new(config)?;
    let feed_repo = FeedRepository::new(db);
    let article_repo = ArticleRepository::new(db);

    let feeds = feed_repo.list_all().await?;
    let mut total_new = 0;

    for feed in feeds {
        tracing::info!("Refreshing feed: {}", feed.local_name);

        match fetcher.fetch(&feed.url, feed.id).await {
            Ok(parsed) => {
                // Update feed metadata
                feed_repo.update_metadata(
                    feed.id,
                    parsed.title.as_deref(),
                    parsed.description.as_deref(),
                    parsed.site_url.as_deref(),
                    parsed.icon_url.as_deref(),
                ).await?;

                // Insert new articles
                let new_count = article_repo.create_many(&parsed.articles).await?;
                total_new += new_count;

                tracing::info!("Feed '{}': {} new articles", feed.local_name, new_count);
            }
            Err(e) => {
                tracing::error!("Failed to fetch feed '{}': {}", feed.local_name, e);
                feed_repo.update_fetch_error(feed.id, &e.to_string()).await?;
            }
        }
    }

    Ok(total_new)
}

/// Clean up articles older than retention period
pub async fn cleanup_old_articles(db: &Database, config: &AppConfig) -> Result<u32> {
    let article_repo = ArticleRepository::new(db);
    let deleted = article_repo.cleanup_old_articles(config.general.article_retention_days).await?;

    if deleted > 0 {
        tracing::info!("Cleaned up {} old articles", deleted);
    }

    Ok(deleted)
}

/// Summarize pending articles using batch processing
pub async fn summarize_pending_articles(
    db: &Database,
    summarizer: Arc<Summarizer>,
    limit: u32,
) -> Result<u32> {
    let article_repo = ArticleRepository::new(db);
    let articles = article_repo.list_unsummarized(limit).await?;

    if articles.is_empty() {
        return Ok(0);
    }

    let min_content_len = summarizer.min_content_length();
    let batch_char_limit = summarizer.batch_char_limit();

    // Filter articles with sufficient content and convert to batch format
    let valid_articles: Vec<_> = articles
        .iter()
        .filter_map(|a| {
            a.content_text.as_ref().and_then(|content| {
                if content.trim().len() >= min_content_len {
                    Some(ArticleForSummary {
                        id: a.id.to_string(),
                        title: a.title.clone(),
                        content: content.clone(),
                    })
                } else {
                    tracing::debug!(
                        "Skipping article '{}': content too short ({} chars, min {})",
                        a.title,
                        content.trim().len(),
                        min_content_len
                    );
                    None
                }
            })
        })
        .collect();

    if valid_articles.is_empty() {
        tracing::info!("No articles with sufficient content to summarize");
        return Ok(0);
    }

    // Split into batches based on character limit
    let batches = create_batches(valid_articles, batch_char_limit);
    tracing::info!(
        "Processing {} articles in {} batch(es)",
        articles.len(),
        batches.len()
    );
    let mut summarized = 0;

    for (batch_idx, batch) in batches.into_iter().enumerate() {
        tracing::debug!("Processing batch {} with {} articles", batch_idx + 1, batch.len());

        match summarizer.batch_summarize(batch).await {
            Ok(results) => {
                for result in results {
                    if let Some(summary) = result.summary {
                        // Parse article ID back to Uuid
                        if let Ok(article_id) = Uuid::parse_str(&result.id) {
                            if let Err(e) = article_repo.update_summary(article_id, &summary).await {
                                tracing::warn!("Failed to save summary for article {}: {}", article_id, e);
                                continue;
                            }

                            // Find original article to extract tags
                            if let Some(article) = articles.iter().find(|a| a.id == article_id) {
                                if let Some(content) = &article.content_text {
                                    if let Ok(tags) = summarizer.extract_tags(content).await {
                                        if let Err(e) = article_repo.add_tags(article_id, &tags, "ai").await {
                                            tracing::warn!("Failed to add tags for article {}: {}", article_id, e);
                                        }
                                    }
                                }
                            }

                            summarized += 1;
                            tracing::debug!("Summarized article ID {}", article_id);
                        }
                    } else if let Some(error) = result.error {
                        tracing::warn!("Batch result error for article {}: {}", result.id, error);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Batch summarization failed: {}", e);
            }
        }
    }

    if summarized > 0 {
        tracing::info!("Summarized {} articles", summarized);
    }

    Ok(summarized)
}

/// Split articles into batches based on character limit
fn create_batches(articles: Vec<ArticleForSummary>, char_limit: usize) -> Vec<Vec<ArticleForSummary>> {
    let mut batches = Vec::new();
    let mut current_batch = Vec::new();
    let mut current_chars = 0;

    // Overhead per article (prompt formatting, ID, title, etc.)
    const OVERHEAD_PER_ARTICLE: usize = 200;
    // Base prompt overhead
    const BASE_OVERHEAD: usize = 500;

    for article in articles {
        let article_chars = article.content.len() + article.title.len() + OVERHEAD_PER_ARTICLE;

        // If adding this article would exceed limit, start new batch
        if !current_batch.is_empty() && current_chars + article_chars > char_limit - BASE_OVERHEAD {
            batches.push(current_batch);
            current_batch = Vec::new();
            current_chars = 0;
        }

        current_chars += article_chars;
        current_batch.push(article);
    }

    // Don't forget the last batch
    if !current_batch.is_empty() {
        batches.push(current_batch);
    }

    batches
}

/// Score and filter articles based on relevance to user interests
/// Stage 2 of the workflow: after summarization is complete
pub async fn score_and_filter_articles(
    db: &Database,
    summarizer: Arc<Summarizer>,
    relevance_threshold: f64,
    min_summarize_length: usize,
) -> Result<(u32, u32)> {
    let article_repo = ArticleRepository::new(db);
    let analyzer = ProfileAnalyzer::new(db);

    // Get user interests from profile
    let interests = analyzer.get_top_tags(TimeWindow::Last30Days, 10).await?;

    // Get unread articles that have summaries (completed Stage 1)
    let summarized_articles = article_repo.list_unread_summarized().await?;

    // Get unread short articles (< min_summarize_length) that don't need summarization
    let all_unread = article_repo.list_unread(1000).await?;
    let short_articles: Vec<_> = all_unread
        .into_iter()
        .filter(|a| {
            // No summary and content is short enough to skip summarization
            a.summary.is_none()
                && a.content_text
                    .as_ref()
                    .map(|c| c.trim().len() < min_summarize_length)
                    .unwrap_or(true)
        })
        .collect();

    // Build scoring candidates
    let mut candidates: Vec<(Uuid, String)> = Vec::new();

    // Summarized articles: use title + summary
    for article in &summarized_articles {
        if let Some(ref summary) = article.summary {
            let content = format!("{}\n\n{}", article.title, summary);
            candidates.push((article.id, content));
        }
    }

    // Short articles: use title + content
    for article in &short_articles {
        let content = if let Some(ref text) = article.content_text {
            format!("{}\n\n{}", article.title, text)
        } else {
            article.title.clone()
        };
        candidates.push((article.id, content));
    }

    if candidates.is_empty() {
        tracing::info!("No articles to score");
        return Ok((0, 0));
    }

    // Convert to ArticleForScoring
    let articles_for_scoring: Vec<ArticleForScoring> = candidates
        .iter()
        .map(|(id, content)| ArticleForScoring {
            id: id.to_string(),
            content: content.clone(),
        })
        .collect();

    // Split into batches
    let batch_char_limit = summarizer.batch_char_limit();
    let batches = create_scoring_batches(articles_for_scoring, batch_char_limit);

    tracing::info!(
        "Scoring {} articles in {} batch(es)",
        candidates.len(),
        batches.len()
    );

    let mut scored = 0u32;
    let mut filtered = 0u32;

    for (batch_idx, batch) in batches.into_iter().enumerate() {
        tracing::debug!(
            "Processing scoring batch {} with {} articles",
            batch_idx + 1,
            batch.len()
        );

        match summarizer.batch_score_relevance(batch, &interests).await {
            Ok(results) => {
                for result in results {
                    if let Ok(article_id) = Uuid::parse_str(&result.id) {
                        if let Some(score) = result.score {
                            scored += 1;

                            if score < relevance_threshold {
                                // Mark low-relevance articles as read (auto-filter)
                                if let Err(e) = article_repo.mark_read(article_id).await {
                                    tracing::warn!(
                                        "Failed to mark article {} as read: {}",
                                        article_id,
                                        e
                                    );
                                    continue;
                                }
                                filtered += 1;
                                tracing::debug!(
                                    "Filtered article {} with score {:.2} (threshold: {:.2})",
                                    article_id,
                                    score,
                                    relevance_threshold
                                );
                            }
                        } else if let Some(error) = result.error {
                            tracing::warn!(
                                "Score result error for article {}: {}",
                                result.id,
                                error
                            );
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Batch scoring failed: {}", e);
            }
        }
    }

    if scored > 0 {
        tracing::info!(
            "Scored {} articles, filtered {} below threshold {:.2}",
            scored,
            filtered,
            relevance_threshold
        );
    }

    Ok((scored, filtered))
}

/// Split articles into batches for scoring based on character limit
fn create_scoring_batches(
    articles: Vec<ArticleForScoring>,
    char_limit: usize,
) -> Vec<Vec<ArticleForScoring>> {
    let mut batches = Vec::new();
    let mut current_batch = Vec::new();
    let mut current_chars = 0;

    // Overhead per article (prompt formatting, ID, etc.)
    const OVERHEAD_PER_ARTICLE: usize = 100;
    // Base prompt overhead
    const BASE_OVERHEAD: usize = 300;

    for article in articles {
        let article_chars = article.content.len() + OVERHEAD_PER_ARTICLE;

        // If adding this article would exceed limit, start new batch
        if !current_batch.is_empty() && current_chars + article_chars > char_limit - BASE_OVERHEAD {
            batches.push(current_batch);
            current_batch = Vec::new();
            current_chars = 0;
        }

        current_chars += article_chars;
        current_batch.push(article);
    }

    // Don't forget the last batch
    if !current_batch.is_empty() {
        batches.push(current_batch);
    }

    batches
}
