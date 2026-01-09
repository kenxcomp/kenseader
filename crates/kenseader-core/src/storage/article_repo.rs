use chrono::{DateTime, Duration, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use super::Database;
use crate::feed::{Article, NewArticle};
use crate::Result;

/// Repository for article CRUD operations
pub struct ArticleRepository<'a> {
    db: &'a Database,
}

#[derive(FromRow)]
struct ArticleRow {
    id: String,
    feed_id: String,
    guid: String,
    url: Option<String>,
    title: String,
    author: Option<String>,
    content: Option<String>,
    content_text: Option<String>,
    summary: Option<String>,
    summary_generated_at: Option<DateTime<Utc>>,
    published_at: Option<DateTime<Utc>>,
    fetched_at: DateTime<Utc>,
    is_read: i32,
    read_at: Option<DateTime<Utc>>,
    is_saved: i32,
    created_at: DateTime<Utc>,
    image_url: Option<String>,
    relevance_score: Option<f64>,
}

impl From<ArticleRow> for Article {
    fn from(row: ArticleRow) -> Self {
        Article {
            id: Uuid::parse_str(&row.id).unwrap_or_default(),
            feed_id: Uuid::parse_str(&row.feed_id).unwrap_or_default(),
            guid: row.guid,
            url: row.url,
            title: row.title,
            author: row.author,
            content: row.content,
            content_text: row.content_text,
            summary: row.summary,
            summary_generated_at: row.summary_generated_at,
            published_at: row.published_at,
            fetched_at: row.fetched_at,
            is_read: row.is_read != 0,
            read_at: row.read_at,
            is_saved: row.is_saved != 0,
            created_at: row.created_at,
            image_url: row.image_url,
            relevance_score: row.relevance_score,
            tags: Vec::new(),
        }
    }
}

impl<'a> ArticleRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a new article (with deduplication by feed_id + guid)
    pub async fn create(&self, new_article: &NewArticle) -> Result<Option<Article>> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        // Try to insert, ignore if duplicate (feed_id, guid)
        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO articles
            (id, feed_id, guid, url, title, author, content, content_text, published_at, fetched_at, created_at, image_url)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(new_article.feed_id.to_string())
        .bind(&new_article.guid)
        .bind(&new_article.url)
        .bind(&new_article.title)
        .bind(&new_article.author)
        .bind(&new_article.content)
        .bind(&new_article.content_text)
        .bind(new_article.published_at)
        .bind(now)
        .bind(now)
        .bind(&new_article.image_url)
        .execute(self.db.pool())
        .await?;

        if result.rows_affected() > 0 {
            self.find_by_id(id).await
        } else {
            sqlx::query(
                r#"
                UPDATE articles
                SET url = COALESCE(?, url),
                    title = ?,
                    author = COALESCE(?, author),
                    content = COALESCE(?, content),
                    content_text = COALESCE(?, content_text),
                    published_at = COALESCE(?, published_at),
                    fetched_at = ?,
                    image_url = COALESCE(?, image_url)
                WHERE feed_id = ? AND guid = ?
                "#,
            )
            .bind(&new_article.url)
            .bind(&new_article.title)
            .bind(&new_article.author)
            .bind(&new_article.content)
            .bind(&new_article.content_text)
            .bind(new_article.published_at)
            .bind(now)
            .bind(&new_article.image_url)
            .bind(new_article.feed_id.to_string())
            .bind(&new_article.guid)
            .execute(self.db.pool())
            .await?;

            // Article already exists
            Ok(None)
        }
    }

    /// Create multiple articles, returning count of newly created
    pub async fn create_many(&self, articles: &[NewArticle]) -> Result<u32> {
        let mut created = 0;

        for article in articles {
            if let Some(_) = self.create(article).await? {
                created += 1;
            }
        }

        Ok(created)
    }

    /// Find an article by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Article>> {
        let row: Option<ArticleRow> = sqlx::query_as(
            r#"
            SELECT id, feed_id, guid, url, title, author, content, content_text,
                   summary, summary_generated_at, published_at, fetched_at,
                   is_read, read_at, is_saved, created_at, image_url, relevance_score
            FROM articles
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(self.db.pool())
        .await?;

        match row {
            Some(r) => {
                let mut article = Article::from(r);
                article.tags = self.get_tags(id).await?;
                Ok(Some(article))
            }
            None => Ok(None),
        }
    }

    /// Get articles for a feed
    pub async fn list_by_feed(&self, feed_id: Uuid, unread_only: bool) -> Result<Vec<Article>> {
        let query = if unread_only {
            r#"
            SELECT id, feed_id, guid, url, title, author, content, content_text,
                   summary, summary_generated_at, published_at, fetched_at,
                   is_read, read_at, is_saved, created_at, image_url, relevance_score
            FROM articles
            WHERE feed_id = ? AND is_read = 0
            ORDER BY published_at DESC, created_at DESC
            "#
        } else {
            r#"
            SELECT id, feed_id, guid, url, title, author, content, content_text,
                   summary, summary_generated_at, published_at, fetched_at,
                   is_read, read_at, is_saved, created_at, image_url, relevance_score
            FROM articles
            WHERE feed_id = ?
            ORDER BY published_at DESC, created_at DESC
            "#
        };

        let rows: Vec<ArticleRow> = sqlx::query_as(query)
            .bind(feed_id.to_string())
            .fetch_all(self.db.pool())
            .await?;

        Ok(rows.into_iter().map(Article::from).collect())
    }

    /// Get all unread articles that have been summarized
    pub async fn list_unread_summarized(&self) -> Result<Vec<Article>> {
        let rows: Vec<ArticleRow> = sqlx::query_as(
            r#"
            SELECT id, feed_id, guid, url, title, author, content, content_text,
                   summary, summary_generated_at, published_at, fetched_at,
                   is_read, read_at, is_saved, created_at, image_url, relevance_score
            FROM articles
            WHERE is_read = 0 AND summary IS NOT NULL
            ORDER BY published_at DESC, created_at DESC
            "#,
        )
        .fetch_all(self.db.pool())
        .await?;

        Ok(rows.into_iter().map(Article::from).collect())
    }

    /// Get all unread articles
    pub async fn list_unread(&self, limit: u32) -> Result<Vec<Article>> {
        let rows: Vec<ArticleRow> = sqlx::query_as(
            r#"
            SELECT id, feed_id, guid, url, title, author, content, content_text,
                   summary, summary_generated_at, published_at, fetched_at,
                   is_read, read_at, is_saved, created_at, image_url, relevance_score
            FROM articles
            WHERE is_read = 0
            ORDER BY published_at DESC, created_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(self.db.pool())
        .await?;

        Ok(rows.into_iter().map(Article::from).collect())
    }

    /// Get articles that need summarization
    /// Only returns unread articles with content_text length >= min_length and no summary
    pub async fn list_unsummarized(&self, limit: u32, min_length: usize) -> Result<Vec<Article>> {
        let rows: Vec<ArticleRow> = sqlx::query_as(
            r#"
            SELECT id, feed_id, guid, url, title, author, content, content_text,
                   summary, summary_generated_at, published_at, fetched_at,
                   is_read, read_at, is_saved, created_at, image_url, relevance_score
            FROM articles
            WHERE summary IS NULL
              AND content_text IS NOT NULL
              AND LENGTH(content_text) >= ?
              AND is_read = 0
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(min_length as i64)
        .bind(limit)
        .fetch_all(self.db.pool())
        .await?;

        Ok(rows.into_iter().map(Article::from).collect())
    }

    /// Check if articles are still unread (for batch processing validation)
    /// Returns only the IDs of articles that are still unread
    pub async fn filter_unread_ids(&self, ids: &[Uuid]) -> Result<Vec<Uuid>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        // Build placeholders for IN clause
        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        let query = format!(
            "SELECT id FROM articles WHERE id IN ({}) AND is_read = 0",
            placeholders.join(", ")
        );

        let mut query_builder = sqlx::query_scalar::<_, String>(&query);
        for id in ids {
            query_builder = query_builder.bind(id.to_string());
        }

        let rows: Vec<String> = query_builder.fetch_all(self.db.pool()).await?;

        Ok(rows
            .into_iter()
            .filter_map(|id| Uuid::parse_str(&id).ok())
            .collect())
    }

    /// Mark an article as read
    pub async fn mark_read(&self, id: Uuid) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE articles
            SET is_read = 1, read_at = ?
            WHERE id = ?
            "#,
        )
        .bind(now)
        .bind(id.to_string())
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Mark an article as unread
    pub async fn mark_unread(&self, id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE articles
            SET is_read = 0, read_at = NULL
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Toggle article saved status
    pub async fn toggle_saved(&self, id: Uuid) -> Result<bool> {
        sqlx::query(
            r#"
            UPDATE articles
            SET is_saved = 1 - is_saved
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .execute(self.db.pool())
        .await?;

        // Return the new saved status
        let row: (i32,) = sqlx::query_as("SELECT is_saved FROM articles WHERE id = ?")
            .bind(id.to_string())
            .fetch_one(self.db.pool())
            .await?;

        Ok(row.0 != 0)
    }

    /// Update article summary
    pub async fn update_summary(&self, id: Uuid, summary: &str) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE articles
            SET summary = ?, summary_generated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(summary)
        .bind(now)
        .bind(id.to_string())
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Update article relevance score
    pub async fn update_relevance_score(&self, id: Uuid, score: f64) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE articles
            SET relevance_score = ?
            WHERE id = ?
            "#,
        )
        .bind(score)
        .bind(id.to_string())
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Add tags to an article
    pub async fn add_tags(&self, article_id: Uuid, tags: &[String], source: &str) -> Result<()> {
        let now = Utc::now();

        for tag in tags {
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO article_tags (article_id, tag, source, created_at)
                VALUES (?, ?, ?, ?)
                "#,
            )
            .bind(article_id.to_string())
            .bind(tag)
            .bind(source)
            .bind(now)
            .execute(self.db.pool())
            .await?;
        }

        Ok(())
    }

    /// Get tags for an article
    pub async fn get_tags(&self, article_id: Uuid) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT tag FROM article_tags WHERE article_id = ?",
        )
        .bind(article_id.to_string())
        .fetch_all(self.db.pool())
        .await?;

        Ok(rows.into_iter().map(|(tag,)| tag).collect())
    }

    /// Delete articles older than specified days (except saved ones)
    pub async fn cleanup_old_articles(&self, retention_days: u32) -> Result<u32> {
        let cutoff = Utc::now() - Duration::days(retention_days as i64);

        let result = sqlx::query(
            r#"
            DELETE FROM articles
            WHERE fetched_at < ? AND is_saved = 0
            "#,
        )
        .bind(cutoff)
        .execute(self.db.pool())
        .await?;

        Ok(result.rows_affected() as u32)
    }

    /// Search articles by title or content
    pub async fn search(&self, query: &str, feed_id: Option<Uuid>) -> Result<Vec<Article>> {
        let search_pattern = format!("%{}%", query);

        let rows: Vec<ArticleRow> = if let Some(fid) = feed_id {
            sqlx::query_as(
                r#"
                SELECT id, feed_id, guid, url, title, author, content, content_text,
                       summary, summary_generated_at, published_at, fetched_at,
                       is_read, read_at, is_saved, created_at, image_url, relevance_score
                FROM articles
                WHERE feed_id = ? AND (title LIKE ? OR content_text LIKE ?)
                ORDER BY published_at DESC
                LIMIT 100
                "#,
            )
            .bind(fid.to_string())
            .bind(&search_pattern)
            .bind(&search_pattern)
            .fetch_all(self.db.pool())
            .await?
        } else {
            sqlx::query_as(
                r#"
                SELECT id, feed_id, guid, url, title, author, content, content_text,
                       summary, summary_generated_at, published_at, fetched_at,
                       is_read, read_at, is_saved, created_at, image_url, relevance_score
                FROM articles
                WHERE title LIKE ? OR content_text LIKE ?
                ORDER BY published_at DESC
                LIMIT 100
                "#,
            )
            .bind(&search_pattern)
            .bind(&search_pattern)
            .fetch_all(self.db.pool())
            .await?
        };

        Ok(rows.into_iter().map(Article::from).collect())
    }
}
