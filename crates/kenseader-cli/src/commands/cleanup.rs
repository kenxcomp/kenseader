use anyhow::Result;

use kenseader_core::{
    scheduler::cleanup_old_articles,
    storage::Database,
    AppConfig,
};

pub async fn run(db: &Database, config: &AppConfig) -> Result<()> {
    println!(
        "Cleaning up articles older than {} days...",
        config.general.article_retention_days
    );

    let deleted = cleanup_old_articles(db, config).await?;

    if deleted > 0 {
        println!("Deleted {} old articles.", deleted);
    } else {
        println!("No articles to clean up.");
    }

    Ok(())
}
