use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Feed parsing error: {0}")]
    FeedParse(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("URL parsing error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("AI provider error: {0}")]
    AiProvider(String),

    #[error("Article not found: {0}")]
    ArticleNotFound(String),

    #[error("Feed not found: {0}")]
    FeedNotFound(String),

    #[error("Invalid RSSHub URL: {0}")]
    InvalidRsshubUrl(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
