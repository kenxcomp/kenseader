use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous,
};
use sqlx::{Pool, Sqlite};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use crate::config::AppConfig;
use crate::Result;

/// Database connection pool wrapper
#[derive(Clone)]
pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    /// Create a new database connection and run migrations
    pub async fn new(config: &AppConfig) -> Result<Self> {
        let db_path = config.database_path();

        // Ensure the data directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Clean up potentially stale lock files (for cloud sync scenarios)
        if db_path.exists() {
            Self::cleanup_stale_locks(&db_path)?;
        }

        let db_url = format!("sqlite:{}", db_path.display());

        tracing::info!("Connecting to database: {}", db_path.display());

        // Use SqliteConnectOptions to set PRAGMAs per-connection.
        // This ensures every connection in the pool has the correct settings,
        // not just the first one — critical for cloud sync (iCloud, Dropbox) scenarios.
        let options = SqliteConnectOptions::from_str(&db_url)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(10))
            .pragma("wal_autocheckpoint", "2000");

        let pool = SqlitePoolOptions::new()
            .max_connections(15)
            .acquire_timeout(Duration::from_secs(10))
            .connect_with(options)
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;

        Ok(db)
    }

    /// Create an in-memory database for testing
    #[cfg(test)]
    pub async fn new_in_memory() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;

        Ok(db)
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<()> {
        tracing::info!("Running database migrations...");

        // Create feeds table
        sqlx::query(MIGRATION_001_FEEDS)
            .execute(&self.pool)
            .await?;

        // Create articles table
        sqlx::query(MIGRATION_002_ARTICLES)
            .execute(&self.pool)
            .await?;

        // Create article tags table
        sqlx::query(MIGRATION_003_ARTICLE_TAGS)
            .execute(&self.pool)
            .await?;

        // Create behavior events table
        sqlx::query(MIGRATION_004_BEHAVIOR_EVENTS)
            .execute(&self.pool)
            .await?;

        // Create user preferences table
        sqlx::query(MIGRATION_005_USER_PREFERENCES)
            .execute(&self.pool)
            .await?;

        // Create article styles table
        sqlx::query(MIGRATION_006_ARTICLE_STYLES)
            .execute(&self.pool)
            .await?;

        // Create indexes
        sqlx::query(MIGRATION_INDEXES)
            .execute(&self.pool)
            .await?;

        // Add image_url column to articles (migration 007)
        if let Err(err) = sqlx::query(MIGRATION_007_ARTICLE_IMAGE_URL)
            .execute(&self.pool)
            .await
        {
            if !is_duplicate_column_error(&err) {
                return Err(err.into());
            }
        }

        // Add relevance_score column to articles (migration 008)
        if let Err(err) = sqlx::query(MIGRATION_008_ARTICLE_RELEVANCE_SCORE)
            .execute(&self.pool)
            .await
        {
            if !is_duplicate_column_error(&err) {
                return Err(err.into());
            }
        }

        tracing::info!("Database migrations completed");
        Ok(())
    }

    /// Get the connection pool
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    /// Clean up potentially stale lock files from cloud sync scenarios
    ///
    /// When using cloud storage (iCloud, Dropbox, etc.), WAL lock files (.db-wal, .db-shm)
    /// may become stale if another device held the lock but is no longer active.
    /// This function removes lock files that haven't been modified in over 30 seconds.
    fn cleanup_stale_locks(db_path: &Path) -> Result<()> {
        let wal_path = db_path.with_extension("db-wal");
        let shm_path = db_path.with_extension("db-shm");

        for lock_path in [&wal_path, &shm_path] {
            if lock_path.exists() {
                if let Ok(metadata) = std::fs::metadata(lock_path) {
                    if let Ok(modified) = metadata.modified() {
                        if modified.elapsed().unwrap_or_default() > Duration::from_secs(30) {
                            tracing::warn!(
                                "Removing potentially stale lock file: {:?}",
                                lock_path
                            );
                            let _ = std::fs::remove_file(lock_path);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn is_duplicate_column_error(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => {
            let message = db_err.message().to_lowercase();
            message.contains("duplicate column")
        }
        _ => false,
    }
}

const MIGRATION_001_FEEDS: &str = r#"
CREATE TABLE IF NOT EXISTS feeds (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL UNIQUE,
    local_name TEXT NOT NULL,
    title TEXT,
    description TEXT,
    site_url TEXT,
    icon_url TEXT,
    last_fetched_at DATETIME,
    fetch_error TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
)
"#;

const MIGRATION_002_ARTICLES: &str = r#"
CREATE TABLE IF NOT EXISTS articles (
    id TEXT PRIMARY KEY,
    feed_id TEXT NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
    guid TEXT NOT NULL,
    url TEXT,
    title TEXT NOT NULL,
    author TEXT,
    content TEXT,
    content_text TEXT,
    summary TEXT,
    summary_generated_at DATETIME,
    published_at DATETIME,
    fetched_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_read INTEGER NOT NULL DEFAULT 0,
    read_at DATETIME,
    is_saved INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(feed_id, guid)
)
"#;

const MIGRATION_003_ARTICLE_TAGS: &str = r#"
CREATE TABLE IF NOT EXISTS article_tags (
    article_id TEXT NOT NULL REFERENCES articles(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'ai',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (article_id, tag)
)
"#;

const MIGRATION_004_BEHAVIOR_EVENTS: &str = r#"
CREATE TABLE IF NOT EXISTS behavior_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    article_id TEXT REFERENCES articles(id) ON DELETE SET NULL,
    feed_id TEXT REFERENCES feeds(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL,
    event_data TEXT,
    reading_duration_ms INTEGER,
    scroll_depth_percent INTEGER,
    context_time_of_day TEXT,
    context_day_of_week INTEGER,
    context_network_type TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
)
"#;

const MIGRATION_005_USER_PREFERENCES: &str = r#"
CREATE TABLE IF NOT EXISTS user_preferences (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    preference_type TEXT NOT NULL,
    preference_key TEXT NOT NULL,
    weight REAL NOT NULL DEFAULT 0.0,
    time_window TEXT NOT NULL,
    computed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(preference_type, preference_key, time_window)
)
"#;

const MIGRATION_006_ARTICLE_STYLES: &str = r#"
CREATE TABLE IF NOT EXISTS article_styles (
    article_id TEXT PRIMARY KEY REFERENCES articles(id) ON DELETE CASCADE,
    style_type TEXT,
    tone TEXT,
    length_category TEXT,
    computed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
)
"#;

const MIGRATION_INDEXES: &str = r#"
CREATE INDEX IF NOT EXISTS idx_feeds_local_name ON feeds(local_name);
CREATE INDEX IF NOT EXISTS idx_articles_feed_id ON articles(feed_id);
CREATE INDEX IF NOT EXISTS idx_articles_is_read ON articles(is_read);
CREATE INDEX IF NOT EXISTS idx_articles_published_at ON articles(published_at DESC);
CREATE INDEX IF NOT EXISTS idx_articles_fetched_at ON articles(fetched_at DESC);
CREATE INDEX IF NOT EXISTS idx_article_tags_tag ON article_tags(tag);
CREATE INDEX IF NOT EXISTS idx_behavior_events_article_id ON behavior_events(article_id);
CREATE INDEX IF NOT EXISTS idx_behavior_events_feed_id ON behavior_events(feed_id);
CREATE INDEX IF NOT EXISTS idx_behavior_events_type ON behavior_events(event_type);
CREATE INDEX IF NOT EXISTS idx_behavior_events_created_at ON behavior_events(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_user_prefs_type ON user_preferences(preference_type);
CREATE INDEX IF NOT EXISTS idx_user_prefs_window ON user_preferences(time_window)
"#;

const MIGRATION_007_ARTICLE_IMAGE_URL: &str = r#"
ALTER TABLE articles ADD COLUMN image_url TEXT
"#;

const MIGRATION_008_ARTICLE_RELEVANCE_SCORE: &str = r#"
ALTER TABLE articles ADD COLUMN relevance_score REAL
"#;
