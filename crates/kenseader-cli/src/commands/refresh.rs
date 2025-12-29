use anyhow::Result;

use kenseader_core::{
    scheduler::refresh_all_feeds,
    storage::Database,
    AppConfig,
};

pub async fn run(db: &Database, config: &AppConfig) -> Result<()> {
    println!("Refreshing all feeds...\n");

    let new_articles = refresh_all_feeds(db, config).await?;

    println!("\nRefresh complete. {} new articles fetched.", new_articles);

    Ok(())
}
