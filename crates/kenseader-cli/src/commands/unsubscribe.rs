use anyhow::Result;

use kenseader_core::storage::{Database, FeedRepository};

pub async fn run(db: &Database, name: &str) -> Result<()> {
    let feed_repo = FeedRepository::new(db);
    let feeds = feed_repo.list_all().await?;

    // Find feed by name
    let feed = feeds.iter().find(|f| f.local_name == name);

    match feed {
        Some(f) => {
            feed_repo.delete(f.id).await?;
            println!("Unsubscribed from: {}", name);
        }
        None => {
            println!("Feed '{}' not found.", name);
            println!("\nAvailable subscriptions:");
            for f in &feeds {
                println!("  - {}", f.local_name);
            }
        }
    }

    Ok(())
}
