use std::sync::Arc;

use tokio::task::JoinSet;

use crate::ai::Summarizer;
use crate::config::AppConfig;
use crate::feed::{Article, FeedFetcher};
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

/// Summarize pending articles
pub async fn summarize_pending_articles(
    db: &Database,
    summarizer: Arc<Summarizer>,
    limit: u32,
) -> Result<u32> {
    let article_repo = ArticleRepository::new(db);
    let articles = article_repo.list_unsummarized(limit).await?;

    let concurrency = summarizer.concurrency();
    let mut summarized = 0;
    let mut join_set: JoinSet<std::result::Result<bool, crate::Error>> = JoinSet::new();
    let mut iter = articles.into_iter();

    fn spawn_task(
        join_set: &mut JoinSet<std::result::Result<bool, crate::Error>>,
        summarizer: Arc<Summarizer>,
        db: Database,
        article: Article,
    ) {
        join_set.spawn(async move {
            let article_repo = ArticleRepository::new(&db);
            if let Some(content) = &article.content_text {
                match summarizer.summarize(content).await {
                    Ok(summary) => {
                        article_repo.update_summary(article.id, &summary).await?;

                        if let Ok(tags) = summarizer.extract_tags(content).await {
                            article_repo.add_tags(article.id, &tags, "ai").await?;
                        }

                        tracing::debug!("Summarized article: {}", article.title);
                        Ok::<bool, crate::Error>(true)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to summarize '{}': {}", article.title, e);
                        Ok::<bool, crate::Error>(false)
                    }
                }
            } else {
                Ok::<bool, crate::Error>(false)
            }
        });
    }

    for _ in 0..concurrency {
        if let Some(article) = iter.next() {
            spawn_task(&mut join_set, Arc::clone(&summarizer), (*db).clone(), article);
        }
    }

    while let Some(result) = join_set.join_next().await {
        let completed = result
            .map_err(|e| crate::Error::AiProvider(format!("Task join error: {}", e)))??;

        if completed {
            summarized += 1;
        }

        if let Some(article) = iter.next() {
            spawn_task(&mut join_set, Arc::clone(&summarizer), (*db).clone(), article);
        }
    }

    if summarized > 0 {
        tracing::info!("Summarized {} articles", summarized);
    }

    Ok(summarized)
}
