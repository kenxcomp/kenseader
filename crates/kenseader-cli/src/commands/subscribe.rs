use anyhow::Result;

use kenseader_core::{
    feed::{FeedFetcher, NewFeed},
    storage::{ArticleRepository, Database, FeedRepository},
    AppConfig,
};

pub async fn run(db: &Database, config: &AppConfig, url: &str, name: &str) -> Result<()> {
    println!("Subscribing to feed: {}", url);

    let fetcher = FeedFetcher::new(config)?;

    // Resolve URL (handles rsshub:// protocol)
    let resolved_url = fetcher.resolve_url(url)?;
    println!("Resolved URL: {}", resolved_url);

    // Check if already subscribed
    let feed_repo = FeedRepository::new(db);
    if feed_repo.find_by_url(&resolved_url).await?.is_some() {
        println!("Already subscribed to this feed.");
        return Ok(());
    }

    // Create the feed entry
    let new_feed = NewFeed {
        url: resolved_url.clone(),
        local_name: name.to_string(),
    };

    let feed = feed_repo.create(&new_feed).await?;
    println!("Created subscription: {} ({})", name, feed.id);

    // Fetch initial articles
    println!("Fetching articles...");
    match fetcher.fetch(&resolved_url, feed.id).await {
        Ok(parsed) => {
            // Update feed metadata
            feed_repo.update_metadata(
                feed.id,
                parsed.title.as_deref(),
                parsed.description.as_deref(),
                parsed.site_url.as_deref(),
                parsed.icon_url.as_deref(),
            ).await?;

            // Insert articles
            let article_repo = ArticleRepository::new(db);
            let count = article_repo.create_many(&parsed.articles).await?;

            println!("Successfully fetched {} articles from '{}'", count, name);

            if let Some(title) = parsed.title {
                println!("Feed title: {}", title);
            }
        }
        Err(e) => {
            println!("Warning: Failed to fetch articles: {}", e);
            println!("The subscription was created, but initial fetch failed.");
            println!("Try running 'kenseader refresh' later.");
        }
    }

    Ok(())
}
