use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub sync: SyncConfig,
    #[serde(default)]
    pub rsshub: RsshubConfig,
    #[serde(default)]
    pub keymap: KeymapConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            ai: AiConfig::default(),
            ui: UiConfig::default(),
            sync: SyncConfig::default(),
            rsshub: RsshubConfig::default(),
            keymap: KeymapConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Data directory path
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,
    /// Article retention in days
    #[serde(default = "default_retention_days")]
    pub article_retention_days: u32,
    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            article_retention_days: default_retention_days(),
            log_level: default_log_level(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// Enable AI summarization
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// AI provider: "claude_cli", "gemini_cli", "codex_cli", "openai", "gemini_api", "claude_api"
    #[serde(default = "default_ai_provider")]
    pub provider: String,
    /// Summary language (e.g., "English", "Chinese", "Japanese")
    #[serde(default = "default_summary_language")]
    pub summary_language: String,
    /// OpenAI API key (for openai provider)
    #[serde(default)]
    pub openai_api_key: Option<String>,
    /// OpenAI model name
    #[serde(default = "default_openai_model")]
    pub openai_model: String,
    /// Gemini API key (for gemini_api provider)
    #[serde(default)]
    pub gemini_api_key: Option<String>,
    /// Gemini model name
    #[serde(default = "default_gemini_model")]
    pub gemini_model: String,
    /// Claude/Anthropic API key (for claude_api provider)
    #[serde(default)]
    pub claude_api_key: Option<String>,
    /// Claude model name
    #[serde(default = "default_claude_model")]
    pub claude_model: String,
    /// Max tokens for summary
    #[serde(default = "default_max_tokens")]
    pub max_summary_tokens: u32,
    /// Concurrent summarization tasks
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    /// Minimum content length (chars) for AI summarization
    #[serde(default = "default_min_summarize_length")]
    pub min_summarize_length: usize,
    /// Maximum summary output length (chars)
    #[serde(default = "default_max_summary_length")]
    pub max_summary_length: usize,
    /// Relevance threshold for article filtering (0.0-1.0)
    #[serde(default = "default_relevance_threshold")]
    pub relevance_threshold: f64,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            provider: default_ai_provider(),
            summary_language: default_summary_language(),
            openai_api_key: None,
            openai_model: default_openai_model(),
            gemini_api_key: None,
            gemini_model: default_gemini_model(),
            claude_api_key: None,
            claude_model: default_claude_model(),
            max_summary_tokens: default_max_tokens(),
            concurrency: default_concurrency(),
            min_summarize_length: default_min_summarize_length(),
            max_summary_length: default_max_summary_length(),
            relevance_threshold: default_relevance_threshold(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Tick rate in milliseconds
    #[serde(default = "default_tick_rate")]
    pub tick_rate_ms: u64,
    /// Show article author
    #[serde(default = "default_true")]
    pub show_author: bool,
    /// Show timestamps
    #[serde(default = "default_true")]
    pub show_timestamps: bool,
    /// Image preview enabled
    #[serde(default = "default_true")]
    pub image_preview: bool,
    /// Theme configuration
    #[serde(default)]
    pub theme: ThemeConfig,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            tick_rate_ms: default_tick_rate(),
            show_author: default_true(),
            show_timestamps: default_true(),
            image_preview: default_true(),
            theme: ThemeConfig::default(),
        }
    }
}

/// Theme configuration
/// Can be specified as a simple string (theme name) or as a full struct with overrides
#[derive(Debug, Clone, Serialize)]
pub struct ThemeConfig {
    /// Theme name (e.g., "gruvbox-dark", "catppuccin-mocha")
    pub name: String,
    /// Optional color overrides for semantic colors
    pub colors: ThemeColorOverrides,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: default_theme_name(),
            colors: ThemeColorOverrides::default(),
        }
    }
}

// Custom deserializer to accept either a string or a struct
impl<'de> Deserialize<'de> for ThemeConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct ThemeConfigVisitor;

        impl<'de> Visitor<'de> for ThemeConfigVisitor {
            type Value = ThemeConfig;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string (theme name) or a map with 'name' and optional 'colors'")
            }

            // Accept a simple string as just the theme name
            fn visit_str<E>(self, value: &str) -> Result<ThemeConfig, E>
            where
                E: de::Error,
            {
                Ok(ThemeConfig {
                    name: value.to_string(),
                    colors: ThemeColorOverrides::default(),
                })
            }

            // Accept a map/struct with 'name' and optional 'colors'
            fn visit_map<M>(self, mut map: M) -> Result<ThemeConfig, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut name: Option<String> = None;
                let mut colors: Option<ThemeColorOverrides> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "name" => {
                            name = Some(map.next_value()?);
                        }
                        "colors" => {
                            colors = Some(map.next_value()?);
                        }
                        _ => {
                            // Ignore unknown fields
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                Ok(ThemeConfig {
                    name: name.unwrap_or_else(default_theme_name),
                    colors: colors.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_any(ThemeConfigVisitor)
    }
}

fn default_theme_name() -> String {
    "gruvbox-dark".to_string()
}

/// Optional color overrides for theme customization
/// Each color is a hex string (e.g., "#ff0000" or "ff0000")
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeColorOverrides {
    /// Primary background
    pub bg0: Option<String>,
    /// Secondary background (slightly lighter)
    pub bg1: Option<String>,
    /// Tertiary background (selection, highlights)
    pub bg2: Option<String>,
    /// Primary foreground
    pub fg0: Option<String>,
    /// Secondary foreground (slightly dimmer)
    pub fg1: Option<String>,
    /// Accent color
    pub accent: Option<String>,
    /// Selection background
    pub selection: Option<String>,
    /// Unread indicator color
    pub unread: Option<String>,
    /// Read indicator color
    pub read: Option<String>,
    /// Error color
    pub error: Option<String>,
    /// Success color
    pub success: Option<String>,
    /// Warning color
    pub warning: Option<String>,
    /// Info color
    pub info: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Auto-refresh interval in seconds (0 = disabled)
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,
    /// Minimum seconds between refreshes for each feed (0 = no limit, refresh all feeds every time)
    #[serde(default = "default_feed_refresh_interval")]
    pub feed_refresh_interval_secs: u64,
    /// Cleanup interval in seconds (remove old articles)
    #[serde(default = "default_cleanup_interval")]
    pub cleanup_interval_secs: u64,
    /// Summarization interval in seconds (AI processing)
    #[serde(default = "default_summarize_interval")]
    pub summarize_interval_secs: u64,
    /// Article filtering interval in seconds
    #[serde(default = "default_filter_interval")]
    pub filter_interval_secs: u64,
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub request_timeout_secs: u64,
    /// Per-domain rate limit delay in milliseconds
    #[serde(default = "default_rate_limit")]
    pub rate_limit_ms: u64,
    /// HTTP proxy URL for feed fetching (e.g., "http://127.0.0.1:7890" or "socks5://127.0.0.1:1080")
    #[serde(default)]
    pub proxy_url: Option<String>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            refresh_interval_secs: default_refresh_interval(),
            feed_refresh_interval_secs: default_feed_refresh_interval(),
            cleanup_interval_secs: default_cleanup_interval(),
            summarize_interval_secs: default_summarize_interval(),
            filter_interval_secs: default_filter_interval(),
            request_timeout_secs: default_timeout(),
            rate_limit_ms: default_rate_limit(),
            proxy_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RsshubConfig {
    /// RSSHub base URL
    #[serde(default = "default_rsshub_base_url")]
    pub base_url: String,
    /// RSSHub access key (for protected instances)
    #[serde(default)]
    pub access_key: Option<String>,
}

impl Default for RsshubConfig {
    fn default() -> Self {
        Self {
            base_url: default_rsshub_base_url(),
            access_key: None,
        }
    }
}

/// Keymap configuration using Vim-style notation
/// Format: "j", "k", "<C-j>" (Ctrl+j), "<S-g>" (Shift+g), "<CR>" (Enter), "<Esc>", "<Tab>", "<Space>"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeymapConfig {
    // Application control
    /// Quit the application
    #[serde(default = "default_key_quit")]
    pub quit: String,

    // Navigation between panels
    /// Focus left panel
    #[serde(default = "default_key_focus_left")]
    pub focus_left: String,
    /// Focus right panel
    #[serde(default = "default_key_focus_right")]
    pub focus_right: String,

    // Navigation within panel
    /// Move cursor down
    #[serde(default = "default_key_move_down")]
    pub move_down: String,
    /// Move cursor up
    #[serde(default = "default_key_move_up")]
    pub move_up: String,

    // Scrolling
    /// Scroll half page down
    #[serde(default = "default_key_scroll_half_down")]
    pub scroll_half_down: String,
    /// Scroll half page up
    #[serde(default = "default_key_scroll_half_up")]
    pub scroll_half_up: String,
    /// Scroll full page down
    #[serde(default = "default_key_scroll_page_down")]
    pub scroll_page_down: String,
    /// Scroll full page up
    #[serde(default = "default_key_scroll_page_up")]
    pub scroll_page_up: String,

    // Article navigation (ArticleDetail only)
    /// Switch to next article (respects UnreadOnly mode)
    #[serde(default = "default_key_next_article")]
    pub next_article: String,
    /// Switch to previous article (respects UnreadOnly mode)
    #[serde(default = "default_key_prev_article")]
    pub prev_article: String,

    // Jump to top/bottom
    /// Jump to top (first item)
    #[serde(default = "default_key_jump_to_top")]
    pub jump_to_top: String,
    /// Jump to bottom (last item)
    #[serde(default = "default_key_jump_to_bottom")]
    pub jump_to_bottom: String,

    // Actions
    /// Select item / Enter article detail
    #[serde(default = "default_key_select")]
    pub select: String,
    /// Open article in browser
    #[serde(default = "default_key_open_browser")]
    pub open_browser: String,
    /// Toggle article saved status
    #[serde(default = "default_key_toggle_saved")]
    pub toggle_saved: String,
    /// Refresh feeds
    #[serde(default = "default_key_refresh")]
    pub refresh: String,
    /// Toggle article read/unread status (or delete feed in Subscriptions)
    #[serde(default = "default_key_toggle_read")]
    pub toggle_read: String,

    // Search
    /// Start forward search
    #[serde(default = "default_key_search_forward")]
    pub search_forward: String,
    /// Start backward search
    #[serde(default = "default_key_search_backward")]
    pub search_backward: String,
    /// Go to next search match
    #[serde(default = "default_key_next_match")]
    pub next_match: String,
    /// Go to previous search match
    #[serde(default = "default_key_prev_match")]
    pub prev_match: String,

    // View mode
    /// Toggle between All and Unread-only view
    #[serde(default = "default_key_toggle_unread_only")]
    pub toggle_unread_only: String,

    // History
    /// Navigate back in history
    #[serde(default = "default_key_history_back")]
    pub history_back: String,
    /// Navigate forward in history
    #[serde(default = "default_key_history_forward")]
    pub history_forward: String,

    // Selection
    /// Toggle selection and move to next
    #[serde(default = "default_key_toggle_select")]
    pub toggle_select: String,
    /// Enter/exit visual selection mode
    #[serde(default = "default_key_visual_mode")]
    pub visual_mode: String,

    // Image viewing
    /// Open focused item in external viewer (image) or browser (link)
    #[serde(default = "default_key_open_item")]
    pub open_item: String,
    /// Enter fullscreen image viewer
    #[serde(default = "default_key_view_image")]
    pub view_image: String,
    /// Focus next item (image or link)
    #[serde(default = "default_key_next_item")]
    pub next_item: String,
    /// Focus previous item (image or link)
    #[serde(default = "default_key_prev_item")]
    pub prev_item: String,
}

impl Default for KeymapConfig {
    fn default() -> Self {
        Self {
            quit: default_key_quit(),
            focus_left: default_key_focus_left(),
            focus_right: default_key_focus_right(),
            move_down: default_key_move_down(),
            move_up: default_key_move_up(),
            scroll_half_down: default_key_scroll_half_down(),
            scroll_half_up: default_key_scroll_half_up(),
            scroll_page_down: default_key_scroll_page_down(),
            scroll_page_up: default_key_scroll_page_up(),
            next_article: default_key_next_article(),
            prev_article: default_key_prev_article(),
            jump_to_top: default_key_jump_to_top(),
            jump_to_bottom: default_key_jump_to_bottom(),
            select: default_key_select(),
            open_browser: default_key_open_browser(),
            toggle_saved: default_key_toggle_saved(),
            refresh: default_key_refresh(),
            toggle_read: default_key_toggle_read(),
            search_forward: default_key_search_forward(),
            search_backward: default_key_search_backward(),
            next_match: default_key_next_match(),
            prev_match: default_key_prev_match(),
            toggle_unread_only: default_key_toggle_unread_only(),
            history_back: default_key_history_back(),
            history_forward: default_key_history_forward(),
            toggle_select: default_key_toggle_select(),
            visual_mode: default_key_visual_mode(),
            open_item: default_key_open_item(),
            view_image: default_key_view_image(),
            next_item: default_key_next_item(),
            prev_item: default_key_prev_item(),
        }
    }
}

// Default keymap values (Vim-style notation)
fn default_key_quit() -> String { "q".to_string() }
fn default_key_focus_left() -> String { "h".to_string() }
fn default_key_focus_right() -> String { "l".to_string() }
fn default_key_move_down() -> String { "j".to_string() }
fn default_key_move_up() -> String { "k".to_string() }
fn default_key_scroll_half_down() -> String { "<C-d>".to_string() }
fn default_key_scroll_half_up() -> String { "<C-u>".to_string() }
fn default_key_scroll_page_down() -> String { "<C-f>".to_string() }
fn default_key_scroll_page_up() -> String { "<C-b>".to_string() }
fn default_key_next_article() -> String { "<C-j>".to_string() }
fn default_key_prev_article() -> String { "<C-k>".to_string() }
fn default_key_jump_to_top() -> String { "gg".to_string() }
fn default_key_jump_to_bottom() -> String { "G".to_string() }
fn default_key_select() -> String { "<CR>".to_string() }
fn default_key_open_browser() -> String { "b".to_string() }
fn default_key_toggle_saved() -> String { "s".to_string() }
fn default_key_refresh() -> String { "r".to_string() }
fn default_key_toggle_read() -> String { "d".to_string() }
fn default_key_search_forward() -> String { "/".to_string() }
fn default_key_search_backward() -> String { "?".to_string() }
fn default_key_next_match() -> String { "n".to_string() }
fn default_key_prev_match() -> String { "N".to_string() }
fn default_key_toggle_unread_only() -> String { "i".to_string() }
fn default_key_history_back() -> String { "u".to_string() }
fn default_key_history_forward() -> String { "<C-r>".to_string() }
fn default_key_toggle_select() -> String { "<Space>".to_string() }
fn default_key_visual_mode() -> String { "v".to_string() }
fn default_key_open_item() -> String { "o".to_string() }
fn default_key_view_image() -> String { "<CR>".to_string() }
fn default_key_next_item() -> String { "<Tab>".to_string() }
fn default_key_prev_item() -> String { "<S-Tab>".to_string() }

fn default_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kenseader")
}

fn default_retention_days() -> u32 {
    3
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_true() -> bool {
    true
}

fn default_ai_provider() -> String {
    "claude_cli".to_string()
}

fn default_summary_language() -> String {
    "English".to_string()
}

fn default_openai_model() -> String {
    "gpt-4o-mini".to_string()
}

fn default_gemini_model() -> String {
    "gemini-2.0-flash".to_string()
}

fn default_claude_model() -> String {
    "claude-sonnet-4-20250514".to_string()
}

fn default_max_tokens() -> u32 {
    150
}

fn default_concurrency() -> usize {
    2
}

fn default_min_summarize_length() -> usize {
    500 // Only summarize articles >= 500 chars
}

fn default_max_summary_length() -> usize {
    150 // Summary output max 150 chars
}

fn default_relevance_threshold() -> f64 {
    0.3 // Articles below this score are auto-marked as read
}

fn default_tick_rate() -> u64 {
    100
}

fn default_refresh_interval() -> u64 {
    3600 // 1 hour - scheduler check interval
}

fn default_feed_refresh_interval() -> u64 {
    43200 // 12 hours - minimum interval between refreshes for each feed
}

fn default_cleanup_interval() -> u64 {
    3600 // 1 hour
}

fn default_summarize_interval() -> u64 {
    60 // 1 minute
}

fn default_filter_interval() -> u64 {
    120 // 2 minutes
}

fn default_timeout() -> u64 {
    30
}

fn default_rate_limit() -> u64 {
    1000
}

fn default_rsshub_base_url() -> String {
    // Use hub.slarker.me as default since rsshub.app is protected by Cloudflare
    "https://hub.slarker.me".to_string()
}

/// Expand tilde (~) in path to user's home directory
fn expand_tilde(path: &std::path::Path) -> PathBuf {
    if let Some(path_str) = path.to_str() {
        if let Some(stripped) = path_str.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(stripped);
            }
        } else if path_str == "~" {
            if let Some(home) = dirs::home_dir() {
                return home;
            }
        }
    }
    path.to_path_buf()
}

impl AppConfig {
    /// Load configuration from file or return defaults
    pub fn load() -> crate::Result<Self> {
        let config_path = Self::config_path();

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            toml::from_str(&content)
                .map_err(|e| crate::Error::Config(e.to_string()))
        } else {
            Ok(Self::default())
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> crate::Result<()> {
        let config_path = Self::config_path();

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::Error::Config(e.to_string()))?;
        std::fs::write(&config_path, content)?;

        Ok(())
    }

    /// Get the configuration file path
    /// Always uses ~/.config/kenseader/config.toml on all platforms
    pub fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("kenseader")
            .join("config.toml")
    }

    /// Get the database file path
    pub fn database_path(&self) -> PathBuf {
        self.data_dir().join("kenseader.db")
    }

    /// Get the Unix socket path for IPC
    pub fn socket_path(&self) -> PathBuf {
        self.data_dir().join("kenseader.sock")
    }

    /// Get the data directory (with tilde expansion)
    pub fn data_dir(&self) -> PathBuf {
        expand_tilde(&self.general.data_dir)
    }
}
