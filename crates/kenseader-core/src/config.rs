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
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            ai: AiConfig::default(),
            ui: UiConfig::default(),
            sync: SyncConfig::default(),
            rsshub: RsshubConfig::default(),
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
    /// AI provider: "claude_cli" or "openai"
    #[serde(default = "default_ai_provider")]
    pub provider: String,
    /// OpenAI API key (for openai provider)
    #[serde(default)]
    pub openai_api_key: Option<String>,
    /// OpenAI model name
    #[serde(default = "default_openai_model")]
    pub openai_model: String,
    /// Max tokens for summary
    #[serde(default = "default_max_tokens")]
    pub max_summary_tokens: u32,
    /// Concurrent summarization tasks
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            provider: default_ai_provider(),
            openai_api_key: None,
            openai_model: default_openai_model(),
            max_summary_tokens: default_max_tokens(),
            concurrency: default_concurrency(),
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
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            tick_rate_ms: default_tick_rate(),
            show_author: default_true(),
            show_timestamps: default_true(),
            image_preview: default_true(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Auto-refresh interval in seconds (0 = disabled)
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub request_timeout_secs: u64,
    /// Per-domain rate limit delay in milliseconds
    #[serde(default = "default_rate_limit")]
    pub rate_limit_ms: u64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            refresh_interval_secs: default_refresh_interval(),
            request_timeout_secs: default_timeout(),
            rate_limit_ms: default_rate_limit(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RsshubConfig {
    /// RSSHub base URL
    #[serde(default = "default_rsshub_base_url")]
    pub base_url: String,
}

impl Default for RsshubConfig {
    fn default() -> Self {
        Self {
            base_url: default_rsshub_base_url(),
        }
    }
}

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

fn default_openai_model() -> String {
    "gpt-4o-mini".to_string()
}

fn default_max_tokens() -> u32 {
    150
}

fn default_concurrency() -> usize {
    2
}

fn default_tick_rate() -> u64 {
    100
}

fn default_refresh_interval() -> u64 {
    300 // 5 minutes
}

fn default_timeout() -> u64 {
    30
}

fn default_rate_limit() -> u64 {
    1000
}

fn default_rsshub_base_url() -> String {
    "https://rsshub.app".to_string()
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
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kenseader")
            .join("config.toml")
    }

    /// Get the database file path
    pub fn database_path(&self) -> PathBuf {
        self.general.data_dir.join("kenseader.db")
    }
}
