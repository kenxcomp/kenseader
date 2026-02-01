use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use super::retry::{execute_with_retry, query_with_retry};
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
        let pool = self.db.pool().clone();
        let id_str = id.to_string();
        let url = new_feed.url.clone();
        let local_name = new_feed.local_name.clone();

        execute_with_retry(|| {
            let pool = pool.clone();
            let id_str = id_str.clone();
            let url = url.clone();
            let local_name = local_name.clone();
            async move {
                sqlx::query(
                    r#"
                    INSERT INTO feeds (id, url, local_name, created_at, updated_at)
                    VALUES (?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&id_str)
                .bind(&url)
                .bind(&local_name)
                .bind(now)
                .bind(now)
                .execute(&pool)
                .await
                .map(|_| ())
            }
        })
        .await?;

        self.find_by_id(id).await?.ok_or_else(|| {
            Error::FeedNotFound(id.to_string())
        })
    }

    /// Find a feed by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Feed>> {
        let pool = self.db.pool().clone();
        let id_str = id.to_string();

        let row: Option<FeedRow> = query_with_retry(|| {
            let pool = pool.clone();
            let id_str = id_str.clone();
            async move {
                sqlx::query_as(
                    r#"
                    SELECT id, url, local_name, title, description, site_url, icon_url,
                           last_fetched_at, fetch_error, created_at, updated_at
                    FROM feeds
                    WHERE id = ?
                    "#,
                )
                .bind(&id_str)
                .fetch_optional(&pool)
                .await
            }
        })
        .await?;

        Ok(row.map(Feed::from))
    }

    /// Find a feed by URL
    pub async fn find_by_url(&self, url: &str) -> Result<Option<Feed>> {
        let pool = self.db.pool().clone();
        let url = url.to_string();

        let row: Option<FeedRow> = query_with_retry(|| {
            let pool = pool.clone();
            let url = url.clone();
            async move {
                sqlx::query_as(
                    r#"
                    SELECT id, url, local_name, title, description, site_url, icon_url,
                           last_fetched_at, fetch_error, created_at, updated_at
                    FROM feeds
                    WHERE url = ?
                    "#,
                )
                .bind(&url)
                .fetch_optional(&pool)
                .await
            }
        })
        .await?;

        Ok(row.map(Feed::from))
    }

    /// Get all feeds with unread counts
    pub async fn list_all(&self) -> Result<Vec<Feed>> {
        let pool = self.db.pool().clone();

        let rows: Vec<FeedRow> = query_with_retry(|| {
            let pool = pool.clone();
            async move {
                sqlx::query_as(
                    r#"
                    SELECT id, url, local_name, title, description, site_url, icon_url,
                           last_fetched_at, fetch_error, created_at, updated_at
                    FROM feeds
                    ORDER BY local_name ASC
                    "#,
                )
                .fetch_all(&pool)
                .await
            }
        })
        .await?;

        let mut feeds: Vec<Feed> = rows.into_iter().map(Feed::from).collect();

        // Fetch unread counts
        for feed in &mut feeds {
            let feed_id_str = feed.id.to_string();
            let count: (i64,) = query_with_retry(|| {
                let pool = pool.clone();
                let feed_id_str = feed_id_str.clone();
                async move {
                    sqlx::query_as(
                        "SELECT COUNT(*) FROM articles WHERE feed_id = ? AND is_read = 0",
                    )
                    .bind(&feed_id_str)
                    .fetch_one(&pool)
                    .await
                }
            })
            .await?;

            feed.unread_count = count.0 as u32;
        }

        Ok(feeds)
    }

    /// List feeds that need refreshing (last_fetched_at is NULL or older than threshold)
    pub async fn list_needs_refresh(&self, min_interval_secs: u64) -> Result<Vec<Feed>> {
        let threshold = Utc::now() - chrono::Duration::seconds(min_interval_secs as i64);
        let pool = self.db.pool().clone();

        let rows: Vec<FeedRow> = query_with_retry(|| {
            let pool = pool.clone();
            async move {
                sqlx::query_as(
                    r#"
                    SELECT id, url, local_name, title, description, site_url, icon_url,
                           last_fetched_at, fetch_error, created_at, updated_at
                    FROM feeds
                    WHERE last_fetched_at IS NULL
                       OR last_fetched_at < ?
                    ORDER BY local_name ASC
                    "#,
                )
                .bind(threshold)
                .fetch_all(&pool)
                .await
            }
        })
        .await?;

        let mut feeds: Vec<Feed> = rows.into_iter().map(Feed::from).collect();

        // Fetch unread counts
        for feed in &mut feeds {
            let feed_id_str = feed.id.to_string();
            let count: (i64,) = query_with_retry(|| {
                let pool = pool.clone();
                let feed_id_str = feed_id_str.clone();
                async move {
                    sqlx::query_as(
                        "SELECT COUNT(*) FROM articles WHERE feed_id = ? AND is_read = 0",
                    )
                    .bind(&feed_id_str)
                    .fetch_one(&pool)
                    .await
                }
            })
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
        let pool = self.db.pool().clone();
        let id_str = id.to_string();
        let title = title.map(|s| s.to_string());
        let description = description.map(|s| s.to_string());
        let site_url = site_url.map(|s| s.to_string());
        let icon_url = icon_url.map(|s| s.to_string());

        execute_with_retry(|| {
            let pool = pool.clone();
            let id_str = id_str.clone();
            let title = title.clone();
            let description = description.clone();
            let site_url = site_url.clone();
            let icon_url = icon_url.clone();
            async move {
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
                .bind(&title)
                .bind(&description)
                .bind(&site_url)
                .bind(&icon_url)
                .bind(now)
                .bind(now)
                .bind(&id_str)
                .execute(&pool)
                .await
                .map(|_| ())
            }
        })
        .await?;

        Ok(())
    }

    /// Update feed fetch error
    pub async fn update_fetch_error(&self, id: Uuid, error: &str) -> Result<()> {
        let now = Utc::now();
        let pool = self.db.pool().clone();
        let id_str = id.to_string();
        let error = error.to_string();

        execute_with_retry(|| {
            let pool = pool.clone();
            let id_str = id_str.clone();
            let error = error.clone();
            async move {
                sqlx::query(
                    r#"
                    UPDATE feeds
                    SET fetch_error = ?,
                        updated_at = ?
                    WHERE id = ?
                    "#,
                )
                .bind(&error)
                .bind(now)
                .bind(&id_str)
                .execute(&pool)
                .await
                .map(|_| ())
            }
        })
        .await?;

        Ok(())
    }

    /// Delete a feed and all its articles
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let pool = self.db.pool().clone();
        let id_str = id.to_string();

        let result = query_with_retry(|| {
            let pool = pool.clone();
            let id_str = id_str.clone();
            async move {
                sqlx::query("DELETE FROM feeds WHERE id = ?")
                    .bind(&id_str)
                    .execute(&pool)
                    .await
            }
        })
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get total feed count
    pub async fn count(&self) -> Result<u32> {
        let pool = self.db.pool().clone();

        let count: (i64,) = query_with_retry(|| {
            let pool = pool.clone();
            async move {
                sqlx::query_as("SELECT COUNT(*) FROM feeds")
                    .fetch_one(&pool)
                    .await
            }
        })
        .await?;

        Ok(count.0 as u32)
    }
}
