use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

use super::retry::{execute_with_retry, query_with_retry};
use super::Database;
use crate::ai::ArticleStyleResult;
use crate::feed::Article;
use crate::Result;

/// Stored article style classification
#[derive(Debug, Clone)]
pub struct ArticleStyle {
    pub article_id: Uuid,
    pub style_type: Option<String>,
    pub tone: Option<String>,
    pub length_category: Option<String>,
    pub computed_at: DateTime<Utc>,
}

/// Repository for article style classifications
pub struct ArticleStyleRepository<'a> {
    db: &'a Database,
}

impl<'a> ArticleStyleRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Insert or update article style classification
    pub async fn upsert(&self, article_id: Uuid, style: &ArticleStyleResult) -> Result<()> {
        let pool = self.db.pool().clone();
        let article_id_str = article_id.to_string();
        let style_type = style.style_type.clone();
        let tone = style.tone.clone();
        let length_category = style.length_category.clone();

        execute_with_retry(|| {
            let pool = pool.clone();
            let article_id_str = article_id_str.clone();
            let style_type = style_type.clone();
            let tone = tone.clone();
            let length_category = length_category.clone();
            async move {
                sqlx::query(
                    r#"
                    INSERT INTO article_styles (article_id, style_type, tone, length_category, computed_at)
                    VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)
                    ON CONFLICT(article_id) DO UPDATE SET
                        style_type = excluded.style_type,
                        tone = excluded.tone,
                        length_category = excluded.length_category,
                        computed_at = CURRENT_TIMESTAMP
                    "#,
                )
                .bind(&article_id_str)
                .bind(&style_type)
                .bind(&tone)
                .bind(&length_category)
                .execute(&pool)
                .await
                .map(|_| ())
            }
        })
        .await?;

        Ok(())
    }

    /// Get style classification for an article
    pub async fn find_by_article(&self, article_id: Uuid) -> Result<Option<ArticleStyle>> {
        let pool = self.db.pool().clone();
        let article_id_str = article_id.to_string();

        let row = query_with_retry(|| {
            let pool = pool.clone();
            let article_id_str = article_id_str.clone();
            async move {
                sqlx::query(
                    r#"
                    SELECT article_id, style_type, tone, length_category, computed_at
                    FROM article_styles
                    WHERE article_id = ?
                    "#,
                )
                .bind(&article_id_str)
                .fetch_optional(&pool)
                .await
            }
        })
        .await?;

        match row {
            Some(row) => {
                let id_str: String = row.get("article_id");
                Ok(Some(ArticleStyle {
                    article_id: Uuid::parse_str(&id_str).unwrap_or_default(),
                    style_type: row.get("style_type"),
                    tone: row.get("tone"),
                    length_category: row.get("length_category"),
                    computed_at: row.get("computed_at"),
                }))
            }
            None => Ok(None),
        }
    }

    /// List articles that don't have style classification yet
    /// Only returns articles that have summaries (i.e., have been processed)
    pub async fn list_unclassified(&self, limit: i64) -> Result<Vec<Article>> {
        let pool = self.db.pool().clone();

        let rows = query_with_retry(|| {
            let pool = pool.clone();
            async move {
                sqlx::query(
                    r#"
                    SELECT a.id, a.feed_id, a.guid, a.title, a.url, a.author, a.content,
                           a.content_text, a.summary, a.summary_generated_at, a.published_at,
                           a.fetched_at, a.is_read, a.read_at, a.is_saved, a.created_at,
                           a.image_url, a.relevance_score
                    FROM articles a
                    LEFT JOIN article_styles s ON a.id = s.article_id
                    WHERE s.article_id IS NULL
                      AND a.summary IS NOT NULL
                      AND length(a.content) >= 500
                    ORDER BY a.fetched_at DESC
                    LIMIT ?
                    "#,
                )
                .bind(limit)
                .fetch_all(&pool)
                .await
            }
        })
        .await?;

        let articles = rows
            .into_iter()
            .map(|row| {
                let id_str: String = row.get("id");
                let feed_id_str: String = row.get("feed_id");
                Article {
                    id: Uuid::parse_str(&id_str).unwrap_or_default(),
                    feed_id: Uuid::parse_str(&feed_id_str).unwrap_or_default(),
                    guid: row.get("guid"),
                    title: row.get("title"),
                    url: row.get("url"),
                    author: row.get("author"),
                    content: row.get("content"),
                    content_text: row.get("content_text"),
                    summary: row.get("summary"),
                    summary_generated_at: row.get("summary_generated_at"),
                    published_at: row.get("published_at"),
                    fetched_at: row.get("fetched_at"),
                    is_read: row.get("is_read"),
                    read_at: row.get("read_at"),
                    is_saved: row.get("is_saved"),
                    created_at: row.get("created_at"),
                    image_url: row.get("image_url"),
                    relevance_score: row.get("relevance_score"),
                    tags: Vec::new(),
                }
            })
            .collect();

        Ok(articles)
    }

    /// Count articles with style classification
    pub async fn count_classified(&self) -> Result<i64> {
        let pool = self.db.pool().clone();

        let row = query_with_retry(|| {
            let pool = pool.clone();
            async move {
                sqlx::query("SELECT COUNT(*) as count FROM article_styles")
                    .fetch_one(&pool)
                    .await
            }
        })
        .await?;

        Ok(row.get("count"))
    }
}
