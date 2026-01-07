use std::io::{self, Write};
use std::path::Path;

use anyhow::Result;

use kenseader_core::{
    feed::{parse_opml_file, FeedFetcher, NewFeed},
    storage::{ArticleRepository, Database, FeedRepository},
    AppConfig,
};

pub async fn run(db: &Database, config: &AppConfig, file_path: &str) -> Result<()> {
    let path = Path::new(file_path);

    if !path.exists() {
        println!("File not found: {}", file_path);
        return Ok(());
    }

    let feeds = parse_opml_file(path)?;
    println!("Found {} feeds in OPML file\n", feeds.len());

    if feeds.is_empty() {
        println!("No feeds found in OPML file.");
        return Ok(());
    }

    let feed_repo = FeedRepository::new(db);
    let article_repo = ArticleRepository::new(db);
    let fetcher = FeedFetcher::new(config)?;

    let mut imported = 0u32;
    let mut skipped = 0u32;
    let mut failed = 0u32;
    let total = feeds.len();

    for (i, opml_feed) in feeds.iter().enumerate() {
        // Truncate name for display if too long
        let display_name = if opml_feed.name.chars().count() > 40 {
            format!("{}...", opml_feed.name.chars().take(37).collect::<String>())
        } else {
            opml_feed.name.clone()
        };

        print!("[{}/{}] {} ... ", i + 1, total, display_name);
        io::stdout().flush().ok();

        // Resolve URL (handles rsshub:// protocol)
        let resolved_url = match fetcher.resolve_url(&opml_feed.url) {
            Ok(url) => url,
            Err(e) => {
                println!("invalid URL: {}", e);
                failed += 1;
                continue;
            }
        };

        // Check if already subscribed
        if feed_repo.find_by_url(&resolved_url).await?.is_some() {
            println!("already subscribed");
            skipped += 1;
            continue;
        }

        // Create feed
        let new_feed = NewFeed {
            url: resolved_url.clone(),
            local_name: opml_feed.name.clone(),
        };

        let feed = match feed_repo.create(&new_feed).await {
            Ok(f) => f,
            Err(e) => {
                println!("failed to create: {}", e);
                failed += 1;
                continue;
            }
        };

        // Fetch articles (optional, don't fail if fetch fails)
        match fetcher.fetch(&resolved_url, feed.id).await {
            Ok(parsed) => {
                // Update feed metadata
                feed_repo
                    .update_metadata(
                        feed.id,
                        parsed.title.as_deref(),
                        parsed.description.as_deref(),
                        parsed.site_url.as_deref(),
                        parsed.icon_url.as_deref(),
                    )
                    .await
                    .ok();

                let count = article_repo.create_many(&parsed.articles).await.unwrap_or(0);
                println!("OK ({} articles)", count);
            }
            Err(_) => {
                println!("OK (fetch pending)");
            }
        }

        imported += 1;
    }

    println!("\nImport complete:");
    println!("  Imported: {}", imported);
    println!("  Skipped (already subscribed): {}", skipped);
    println!("  Failed: {}", failed);

    Ok(())
}
