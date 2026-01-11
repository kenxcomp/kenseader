use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;
use uuid::Uuid;

use crate::ai::{ArticleForScoring, ArticleForSummary, Summarizer};
use crate::config::AppConfig;
use crate::feed::FeedFetcher;
use crate::profile::{ProfileAnalyzer, TimeWindow};
use crate::storage::{ArticleRepository, ArticleStyleRepository, Database, FeedRepository};
use crate::Result;

/// Truncate a string to a maximum number of characters (respecting char boundaries)
fn truncate_chars(input: &str, max_chars: usize) -> &str {
    match input.char_indices().nth(max_chars) {
        Some((idx, _)) => &input[..idx],
        None => input,
    }
}

/// Refresh feeds and fetch new articles
/// Uses smart refresh: only refreshes feeds that haven't been fetched recently
pub async fn refresh_all_feeds(db: &Database, config: &AppConfig) -> Result<u32> {
    let fetcher = FeedFetcher::new(config)?;
    let feed_repo = FeedRepository::new(db);
    let article_repo = ArticleRepository::new(db);

    // Smart refresh: only get feeds that need refreshing
    let feeds = if config.sync.feed_refresh_interval_secs > 0 {
        let needs_refresh = feed_repo.list_needs_refresh(config.sync.feed_refresh_interval_secs).await?;
        let total_feeds = feed_repo.count().await?;
        if needs_refresh.is_empty() {
            tracing::debug!("No feeds need refreshing (all {} feeds are up to date)", total_feeds);
            return Ok(0);
        }
        tracing::info!(
            "Smart refresh: {} of {} feeds need refreshing (interval: {} hours)",
            needs_refresh.len(),
            total_feeds,
            config.sync.feed_refresh_interval_secs / 3600
        );
        needs_refresh
    } else {
        // feed_refresh_interval_secs = 0 means refresh all feeds every time
        feed_repo.list_all().await?
    };

    let mut total_new = 0;
    let rate_limit = Duration::from_millis(config.sync.rate_limit_ms);

    for (idx, feed) in feeds.iter().enumerate() {
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

        // Apply rate limit between requests (skip delay after last feed)
        if rate_limit.as_millis() > 0 && idx < feeds.len() - 1 {
            sleep(rate_limit).await;
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

/// Maximum content length per article (truncate if longer)
const CONTENT_TRUNCATE_LIMIT: usize = 4000;

/// Maximum articles to fetch in one query (safety limit)
const MAX_ARTICLES_PER_CYCLE: u32 = 500;

/// Summarize pending articles using batch processing
/// Batch size is dynamically determined by token limit (~100k tokens)
/// Before each batch, re-checks article status to skip already-read articles
pub async fn summarize_pending_articles(
    db: &Database,
    summarizer: Arc<Summarizer>,
) -> Result<u32> {
    let article_repo = ArticleRepository::new(db);
    let min_content_len = summarizer.min_content_length();
    let batch_char_limit = summarizer.batch_char_limit();

    // Fetch all unread articles without summary
    let articles = article_repo.list_unsummarized(MAX_ARTICLES_PER_CYCLE, min_content_len).await?;

    if articles.is_empty() {
        return Ok(0);
    }

    // Convert articles to batch format with content truncation
    // Store as mutable for dynamic re-batching
    let mut pending_articles: Vec<_> = articles
        .iter()
        .filter_map(|a| {
            a.content_text.as_ref().map(|content| {
                // Truncate content to CONTENT_TRUNCATE_LIMIT characters
                let truncated_content = truncate_chars(content, CONTENT_TRUNCATE_LIMIT);
                ArticleForSummary {
                    id: a.id.to_string(),
                    title: a.title.clone(),
                    content: truncated_content.to_string(),
                }
            })
        })
        .collect();

    if pending_articles.is_empty() {
        tracing::info!("No articles with sufficient content to summarize");
        return Ok(0);
    }

    let total_articles = pending_articles.len();
    tracing::info!("Found {} articles to summarize", total_articles);

    let mut summarized = 0;
    let mut batch_idx = 0;

    // Process articles in batches, re-checking status before each batch
    while !pending_articles.is_empty() {
        batch_idx += 1;

        // Re-check which articles are still unread before processing
        let pending_ids: Vec<Uuid> = pending_articles
            .iter()
            .filter_map(|a| Uuid::parse_str(&a.id).ok())
            .collect();

        let unread_ids = article_repo.filter_unread_ids(&pending_ids).await?;

        // Filter out articles that have been marked as read
        let original_count = pending_articles.len();
        pending_articles.retain(|a| {
            Uuid::parse_str(&a.id)
                .map(|id| unread_ids.contains(&id))
                .unwrap_or(false)
        });

        let filtered_count = original_count - pending_articles.len();
        if filtered_count > 0 {
            tracing::info!(
                "Batch {}: Filtered out {} already-read articles",
                batch_idx,
                filtered_count
            );
        }

        if pending_articles.is_empty() {
            tracing::info!("No more unread articles to summarize");
            break;
        }

        // Create a single batch from pending articles (up to char limit)
        let batch = create_single_batch(&mut pending_articles, batch_char_limit);

        if batch.is_empty() {
            continue;
        }

        tracing::debug!(
            "Processing batch {} with {} articles ({} remaining)",
            batch_idx,
            batch.len(),
            pending_articles.len()
        );

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
                tracing::error!("Batch {} summarization failed: {}", batch_idx, e);
                // Continue with next batch instead of failing completely
            }
        }
    }

    if summarized > 0 {
        tracing::info!("Summarized {} articles in {} batch(es)", summarized, batch_idx);
    }

    Ok(summarized)
}

/// Create a single batch from pending articles (removes articles from input vector)
/// Returns articles that fit within the character limit
fn create_single_batch(
    pending: &mut Vec<ArticleForSummary>,
    char_limit: usize,
) -> Vec<ArticleForSummary> {
    const OVERHEAD_PER_ARTICLE: usize = 200;
    const BASE_OVERHEAD: usize = 1000;

    let effective_limit = char_limit.saturating_sub(BASE_OVERHEAD);
    let mut batch = Vec::new();
    let mut current_chars = 0;
    let mut indices_to_remove = Vec::new();

    for (idx, article) in pending.iter().enumerate() {
        let article_chars = article.content.len() + article.title.len() + OVERHEAD_PER_ARTICLE;

        // If adding this article would exceed limit, stop
        if !batch.is_empty() && current_chars + article_chars > effective_limit {
            break;
        }

        current_chars += article_chars;
        indices_to_remove.push(idx);
    }

    // Remove articles from pending in reverse order (to preserve indices)
    for &idx in indices_to_remove.iter().rev() {
        batch.push(pending.remove(idx));
    }

    // Reverse to maintain original order
    batch.reverse();

    if !batch.is_empty() {
        tracing::debug!(
            "Created batch: {} articles, {} chars (limit: {})",
            batch.len(),
            current_chars,
            effective_limit
        );
    }

    batch
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

    // Compute user preferences from behavior events before scoring
    if let Err(e) = analyzer.compute_preferences().await {
        tracing::warn!("Failed to compute user preferences: {}", e);
    }

    // Get user interests from profile
    let interests = analyzer.get_top_tags(TimeWindow::Last30Days, 10).await?;
    if interests.is_empty() {
        tracing::info!("No user interests found - articles will pass through without filtering");
    } else {
        tracing::info!("Scoring with {} user interests: {:?}", interests.len(), interests);
    }

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

                            // Save score to database for debugging
                            if let Err(e) = article_repo.update_relevance_score(article_id, score).await {
                                tracing::warn!("Failed to save relevance score for {}: {}", article_id, e);
                            }

                            // Log each article's score for debugging
                            tracing::info!(
                                "Article {} scored {:.2} (threshold: {:.2})",
                                article_id,
                                score,
                                relevance_threshold
                            );

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
                                tracing::info!(
                                    "FILTERED: Article {} with score {:.2} < threshold {:.2}",
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

/// Classify pending articles that have summaries but no style classification
/// Stage 3 of the workflow: after scoring is complete
pub async fn classify_pending_articles(
    db: &Database,
    summarizer: Arc<Summarizer>,
    batch_size: usize,
) -> Result<u32> {
    let style_repo = ArticleStyleRepository::new(db);
    let unclassified = style_repo.list_unclassified(batch_size as i64).await?;

    if unclassified.is_empty() {
        tracing::debug!("No articles pending style classification");
        return Ok(0);
    }

    tracing::info!(
        "Classifying {} articles for style/tone",
        unclassified.len()
    );

    let mut classified = 0u32;

    for article in unclassified {
        // Use content for classification
        let content = article.content.as_deref().unwrap_or(&article.title);

        match summarizer.classify_style(content).await {
            Ok(result) => {
                if let Err(e) = style_repo.upsert(article.id, &result).await {
                    tracing::warn!(
                        "Failed to save style classification for article {}: {}",
                        article.id,
                        e
                    );
                    continue;
                }
                classified += 1;
                tracing::debug!(
                    "Classified article '{}': style={}, tone={}, length={}",
                    article.title,
                    result.style_type,
                    result.tone,
                    result.length_category
                );
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to classify article '{}': {}",
                    article.title,
                    e
                );
            }
        }
    }

    if classified > 0 {
        tracing::info!("Classified {} articles", classified);
    }

    Ok(classified)
}
