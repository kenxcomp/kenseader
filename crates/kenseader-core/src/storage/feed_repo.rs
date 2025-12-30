use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use super::Database;
use crate::feed::{Feed, NewFeed};
use crate::{Error, Result};

/// Repository for feed CRUD operations
pub struct FeedRepository<'a> {
    db: &'a Database,
}

#[derive(FromRow)]
struct FeedRow {
    id: String,
    url: String,
    local_name: String,
    title: Option<String>,
    description: Option<String>,
    site_url: Option<String>,
    icon_url: Option<String>,
    last_fetched_at: Option<DateTime<Utc>>,
    fetch_error: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<FeedRow> for Feed {
    fn from(row: FeedRow) -> Self {
        Feed {
            id: Uuid::parse_str(&row.id).unwrap_or_default(),
            url: row.url,
            local_name: row.local_name,
            title: row.title,
            description: row.description,
            site_url: row.site_url,
            icon_url: row.icon_url,
            last_fetched_at: row.last_fetched_at,
            fetch_error: row.fetch_error,
            created_at: row.created_at,
            updated_at: row.updated_at,
            unread_count: 0,
        }
    }
}

impl<'a> FeedRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a new feed subscription
    pub async fn create(&self, new_feed: &NewFeed) -> Result<Feed> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO feeds (id, url, local_name, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(&new_feed.url)
        .bind(&new_feed.local_name)
        .bind(now)
        .bind(now)
        .execute(self.db.pool())
        .await?;

        self.find_by_id(id).await?.ok_or_else(|| {
            Error::FeedNotFound(id.to_string())
        })
    }

    /// Find a feed by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Feed>> {
        let row: Option<FeedRow> = sqlx::query_as(
            r#"
            SELECT id, url, local_name, title, description, site_url, icon_url,
                   last_fetched_at, fetch_error, created_at, updated_at
            FROM feeds
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(self.db.pool())
        .await?;

        Ok(row.map(Feed::from))
    }

    /// Find a feed by URL
    pub async fn find_by_url(&self, url: &str) -> Result<Option<Feed>> {
        let row: Option<FeedRow> = sqlx::query_as(
            r#"
            SELECT id, url, local_name, title, description, site_url, icon_url,
                   last_fetched_at, fetch_error, created_at, updated_at
            FROM feeds
            WHERE url = ?
            "#,
        )
        .bind(url)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(row.map(Feed::from))
    }

    /// Get all feeds with unread counts
    pub async fn list_all(&self) -> Result<Vec<Feed>> {
        let rows: Vec<FeedRow> = sqlx::query_as(
            r#"
            SELECT id, url, local_name, title, description, site_url, icon_url,
                   last_fetched_at, fetch_error, created_at, updated_at
            FROM feeds
            ORDER BY local_name ASC
            "#,
        )
        .fetch_all(self.db.pool())
        .await?;

        let mut feeds: Vec<Feed> = rows.into_iter().map(Feed::from).collect();

        // Fetch unread counts
        for feed in &mut feeds {
            let count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM articles WHERE feed_id = ? AND is_read = 0",
            )
            .bind(feed.id.to_string())
            .fetch_one(self.db.pool())
            .await?;

            feed.unread_count = count.0 as u32;
        }

        Ok(feeds)
    }

    /// Update feed metadata after successful fetch
    pub async fn update_metadata(
        &self,
        id: Uuid,
        title: Option<&str>,
        description: Option<&str>,
        site_url: Option<&str>,
        icon_url: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE feeds
            SET title = COALESCE(?, title),
                description = COALESCE(?, description),
                site_url = COALESCE(?, site_url),
                icon_url = COALESCE(?, icon_url),
                last_fetched_at = ?,
                fetch_error = NULL,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(title)
        .bind(description)
        .bind(site_url)
        .bind(icon_url)
        .bind(now)
        .bind(now)
        .bind(id.to_string())
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Update feed fetch error
    pub async fn update_fetch_error(&self, id: Uuid, error: &str) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE feeds
            SET fetch_error = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(error)
        .bind(now)
        .bind(id.to_string())
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Delete a feed and all its articles
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM feeds WHERE id = ?")
            .bind(id.to_string())
            .execute(self.db.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get total feed count
    pub async fn count(&self) -> Result<u32> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM feeds")
            .fetch_one(self.db.pool())
            .await?;

        Ok(count.0 as u32)
    }
}
