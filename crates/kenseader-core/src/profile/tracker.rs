use chrono::{Datelike, Timelike, Utc};
use uuid::Uuid;

use super::models::{BehaviorEventType, TimeOfDay};
use crate::storage::Database;
use crate::Result;

/// Tracks user behavior events
pub struct BehaviorTracker<'a> {
    db: &'a Database,
}

impl<'a> BehaviorTracker<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Record a behavior event
    pub async fn record_event(
        &self,
        article_id: Option<Uuid>,
        feed_id: Option<Uuid>,
        event_type: BehaviorEventType,
        reading_duration_ms: Option<i64>,
        scroll_depth_percent: Option<u8>,
    ) -> Result<()> {
        let now = Utc::now();
        let time_of_day = TimeOfDay::from_hour(now.hour());
        let day_of_week = now.weekday().num_days_from_monday() as i32;

        sqlx::query(
            r#"
            INSERT INTO behavior_events
            (article_id, feed_id, event_type, reading_duration_ms, scroll_depth_percent,
             context_time_of_day, context_day_of_week, context_network_type, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(article_id.map(|id| id.to_string()))
        .bind(feed_id.map(|id| id.to_string()))
        .bind(event_type.as_str())
        .bind(reading_duration_ms)
        .bind(scroll_depth_percent.map(|p| p as i32))
        .bind(time_of_day.as_str())
        .bind(day_of_week)
        .bind("unknown") // Network type - could be detected in future
        .bind(now)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Record article exposure (seen in list)
    pub async fn record_exposure(&self, article_id: Uuid, feed_id: Uuid) -> Result<()> {
        self.record_event(
            Some(article_id),
            Some(feed_id),
            BehaviorEventType::Exposure,
            None,
            None,
        )
        .await
    }

    /// Record article click (selected in list)
    pub async fn record_click(&self, article_id: Uuid, feed_id: Uuid) -> Result<()> {
        self.record_event(
            Some(article_id),
            Some(feed_id),
            BehaviorEventType::Click,
            None,
            None,
        )
        .await
    }

    /// Record read start
    pub async fn record_read_start(&self, article_id: Uuid, feed_id: Uuid) -> Result<()> {
        self.record_event(
            Some(article_id),
            Some(feed_id),
            BehaviorEventType::ReadStart,
            None,
            None,
        )
        .await
    }

    /// Record read complete with duration and scroll depth
    pub async fn record_read_complete(
        &self,
        article_id: Uuid,
        feed_id: Uuid,
        duration_ms: i64,
        scroll_depth: u8,
    ) -> Result<()> {
        self.record_event(
            Some(article_id),
            Some(feed_id),
            BehaviorEventType::ReadComplete,
            Some(duration_ms),
            Some(scroll_depth),
        )
        .await
    }

    /// Record save/bookmark action
    pub async fn record_save(&self, article_id: Uuid, feed_id: Uuid) -> Result<()> {
        self.record_event(
            Some(article_id),
            Some(feed_id),
            BehaviorEventType::Save,
            None,
            None,
        )
        .await
    }

    /// Record repeat view
    pub async fn record_repeat_view(&self, article_id: Uuid, feed_id: Uuid) -> Result<()> {
        self.record_event(
            Some(article_id),
            Some(feed_id),
            BehaviorEventType::ViewRepeat,
            None,
            None,
        )
        .await
    }
}
