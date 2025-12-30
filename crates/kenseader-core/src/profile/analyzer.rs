use chrono::{Duration, Utc};

use super::models::{PreferenceType, TimeWindow};
use crate::storage::Database;
use crate::Result;

/// Analyzes user behavior to compute preferences
pub struct ProfileAnalyzer<'a> {
    db: &'a Database,
}

impl<'a> ProfileAnalyzer<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Compute and store user preferences for all time windows
    pub async fn compute_preferences(&self) -> Result<()> {
        self.compute_tag_affinities().await?;
        self.compute_feed_affinities().await?;
        self.compute_time_preferences().await?;
        Ok(())
    }

    /// Get top tags by affinity for a time window
    pub async fn get_top_tags(&self, window: TimeWindow, limit: u32) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT preference_key
            FROM user_preferences
            WHERE preference_type = ? AND time_window = ?
            ORDER BY weight DESC
            LIMIT ?
            "#,
        )
        .bind(PreferenceType::TagAffinity.as_str())
        .bind(window.as_str())
        .bind(limit)
        .fetch_all(self.db.pool())
        .await?;

        Ok(rows.into_iter().map(|(k,)| k).collect())
    }

    /// Compute tag affinities from behavior events
    async fn compute_tag_affinities(&self) -> Result<()> {
        for window in [TimeWindow::Recent5Min, TimeWindow::Last1Day, TimeWindow::Last30Days] {
            let cutoff = match window {
                TimeWindow::Recent5Min => Utc::now() - Duration::minutes(5),
                TimeWindow::Last1Day => Utc::now() - Duration::days(1),
                TimeWindow::Last30Days => Utc::now() - Duration::days(30),
            };

            // Get weighted tag scores from events
            let rows: Vec<(String, f64)> = sqlx::query_as(
                r#"
                SELECT at.tag, SUM(
                    CASE be.event_type
                        WHEN 'exposure' THEN 0.1
                        WHEN 'click' THEN 1.0
                        WHEN 'read_start' THEN 1.5
                        WHEN 'read_complete' THEN 3.0
                        WHEN 'save' THEN 5.0
                        WHEN 'view_repeat' THEN 4.0
                        ELSE 0.5
                    END
                ) as weight
                FROM behavior_events be
                JOIN article_tags at ON be.article_id = at.article_id
                WHERE be.created_at >= ?
                GROUP BY at.tag
                ORDER BY weight DESC
                LIMIT 50
                "#,
            )
            .bind(cutoff)
            .fetch_all(self.db.pool())
            .await?;

            // Update preferences
            let now = Utc::now();
            for (tag, weight) in rows {
                sqlx::query(
                    r#"
                    INSERT OR REPLACE INTO user_preferences
                    (preference_type, preference_key, weight, time_window, computed_at)
                    VALUES (?, ?, ?, ?, ?)
                    "#,
                )
                .bind(PreferenceType::TagAffinity.as_str())
                .bind(&tag)
                .bind(weight)
                .bind(window.as_str())
                .bind(now)
                .execute(self.db.pool())
                .await?;
            }
        }

        Ok(())
    }

    /// Compute feed affinities from behavior events
    async fn compute_feed_affinities(&self) -> Result<()> {
        for window in [TimeWindow::Recent5Min, TimeWindow::Last1Day, TimeWindow::Last30Days] {
            let cutoff = match window {
                TimeWindow::Recent5Min => Utc::now() - Duration::minutes(5),
                TimeWindow::Last1Day => Utc::now() - Duration::days(1),
                TimeWindow::Last30Days => Utc::now() - Duration::days(30),
            };

            let rows: Vec<(String, f64)> = sqlx::query_as(
                r#"
                SELECT feed_id, SUM(
                    CASE event_type
                        WHEN 'exposure' THEN 0.1
                        WHEN 'click' THEN 1.0
                        WHEN 'read_start' THEN 1.5
                        WHEN 'read_complete' THEN 3.0
                        WHEN 'save' THEN 5.0
                        WHEN 'view_repeat' THEN 4.0
                        ELSE 0.5
                    END
                ) as weight
                FROM behavior_events
                WHERE feed_id IS NOT NULL AND created_at >= ?
                GROUP BY feed_id
                ORDER BY weight DESC
                "#,
            )
            .bind(cutoff)
            .fetch_all(self.db.pool())
            .await?;

            let now = Utc::now();
            for (feed_id, weight) in rows {
                sqlx::query(
                    r#"
                    INSERT OR REPLACE INTO user_preferences
                    (preference_type, preference_key, weight, time_window, computed_at)
                    VALUES (?, ?, ?, ?, ?)
                    "#,
                )
                .bind(PreferenceType::FeedAffinity.as_str())
                .bind(&feed_id)
                .bind(weight)
                .bind(window.as_str())
                .bind(now)
                .execute(self.db.pool())
                .await?;
            }
        }

        Ok(())
    }

    /// Compute time of day preferences
    async fn compute_time_preferences(&self) -> Result<()> {
        for window in [TimeWindow::Last1Day, TimeWindow::Last30Days] {
            let cutoff = match window {
                TimeWindow::Recent5Min => Utc::now() - Duration::minutes(5),
                TimeWindow::Last1Day => Utc::now() - Duration::days(1),
                TimeWindow::Last30Days => Utc::now() - Duration::days(30),
            };

            let rows: Vec<(String, f64)> = sqlx::query_as(
                r#"
                SELECT context_time_of_day, COUNT(*) as count
                FROM behavior_events
                WHERE event_type IN ('click', 'read_complete', 'save') AND created_at >= ?
                GROUP BY context_time_of_day
                "#,
            )
            .bind(cutoff)
            .fetch_all(self.db.pool())
            .await?;

            let now = Utc::now();
            for (time_of_day, count) in rows {
                sqlx::query(
                    r#"
                    INSERT OR REPLACE INTO user_preferences
                    (preference_type, preference_key, weight, time_window, computed_at)
                    VALUES (?, ?, ?, ?, ?)
                    "#,
                )
                .bind(PreferenceType::TimePreference.as_str())
                .bind(&time_of_day)
                .bind(count)
                .bind(window.as_str())
                .bind(now)
                .execute(self.db.pool())
                .await?;
            }
        }

        Ok(())
    }
}
