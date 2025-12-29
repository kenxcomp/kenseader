use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Types of user behavior events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorEventType {
    /// Article appeared in the viewport
    Exposure,
    /// User clicked/selected the article
    Click,
    /// User started reading
    ReadStart,
    /// User finished reading (scrolled to end)
    ReadComplete,
    /// Scroll event with depth tracking
    Scroll,
    /// User saved/bookmarked the article
    Save,
    /// User shared the article (future feature)
    Share,
    /// User viewed the article again
    ViewRepeat,
}

impl BehaviorEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Exposure => "exposure",
            Self::Click => "click",
            Self::ReadStart => "read_start",
            Self::ReadComplete => "read_complete",
            Self::Scroll => "scroll",
            Self::Save => "save",
            Self::Share => "share",
            Self::ViewRepeat => "view_repeat",
        }
    }

    /// Get the weight of this event type for preference calculation
    pub fn weight(&self) -> f64 {
        match self {
            Self::Exposure => 0.1,
            Self::Click => 1.0,
            Self::ReadStart => 1.5,
            Self::ReadComplete => 3.0,
            Self::Scroll => 0.5,
            Self::Save => 5.0,
            Self::Share => 5.0,
            Self::ViewRepeat => 4.0,
        }
    }
}

/// Time of day categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeOfDay {
    /// 6:00 - 12:00
    Morning,
    /// 12:00 - 18:00
    Afternoon,
    /// 18:00 - 22:00
    Evening,
    /// 22:00 - 6:00
    Night,
}

impl TimeOfDay {
    pub fn from_hour(hour: u32) -> Self {
        match hour {
            6..=11 => Self::Morning,
            12..=17 => Self::Afternoon,
            18..=21 => Self::Evening,
            _ => Self::Night,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Morning => "morning",
            Self::Afternoon => "afternoon",
            Self::Evening => "evening",
            Self::Night => "night",
        }
    }
}

/// Time windows for preference aggregation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeWindow {
    /// Last 5 minutes
    Recent5Min,
    /// Last 24 hours
    Last1Day,
    /// Last 30 days
    Last30Days,
}

impl TimeWindow {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Recent5Min => "5min",
            Self::Last1Day => "1day",
            Self::Last30Days => "30days",
        }
    }
}

/// A user behavior event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorEvent {
    pub id: i64,
    pub article_id: Option<Uuid>,
    pub feed_id: Option<Uuid>,
    pub event_type: BehaviorEventType,
    pub event_data: Option<serde_json::Value>,
    pub reading_duration_ms: Option<i64>,
    pub scroll_depth_percent: Option<u8>,
    pub context_time_of_day: TimeOfDay,
    pub context_day_of_week: u8,
    pub context_network_type: String,
    pub created_at: DateTime<Utc>,
}

/// Preference type categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreferenceType {
    /// Affinity for specific tags
    TagAffinity,
    /// Affinity for specific feeds
    FeedAffinity,
    /// Preference for time of day
    TimePreference,
    /// Preference for article style
    StylePreference,
}

impl PreferenceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TagAffinity => "tag_affinity",
            Self::FeedAffinity => "feed_affinity",
            Self::TimePreference => "time_preference",
            Self::StylePreference => "style_preference",
        }
    }
}

/// A computed user preference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreference {
    pub preference_type: PreferenceType,
    pub preference_key: String,
    pub weight: f64,
    pub time_window: TimeWindow,
    pub computed_at: DateTime<Utc>,
}
