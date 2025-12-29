use anyhow::Result;

use kenseader_core::storage::{Database, FeedRepository};

pub async fn run(db: &Database) -> Result<()> {
    let feed_repo = FeedRepository::new(db);
    let feeds = feed_repo.list_all().await?;

    if feeds.is_empty() {
        println!("No subscriptions yet.");
        println!("\nTo subscribe to a feed, run:");
        println!("  kenseader -s <url> -n <name>");
        return Ok(());
    }

    println!("Subscriptions ({}):\n", feeds.len());

    for feed in &feeds {
        let unread = if feed.unread_count > 0 {
            format!(" ({} unread)", feed.unread_count)
        } else {
            String::new()
        };

        let error = if let Some(err) = &feed.fetch_error {
            format!(" [ERROR: {}]", err)
        } else {
            String::new()
        };

        let title = feed.title.as_deref().unwrap_or("(no title)");

        println!("  {} - {}{}{}", feed.local_name, title, unread, error);
        println!("    URL: {}", feed.url);
        if let Some(last) = feed.last_fetched_at {
            println!("    Last fetched: {}", last.format("%Y-%m-%d %H:%M"));
        }
        println!();
    }

    Ok(())
}
