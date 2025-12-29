use chrono::{DateTime, Utc};
use feed_rs::parser;
use uuid::Uuid;

use super::models::NewArticle;
use crate::{Error, Result};

/// Parsed feed data from RSS/Atom content
pub struct ParsedFeed {
    pub title: Option<String>,
    pub description: Option<String>,
    pub site_url: Option<String>,
    pub icon_url: Option<String>,
    pub articles: Vec<NewArticle>,
}

/// Parse RSS/Atom feed content into structured data
pub fn parse_feed(content: &[u8], feed_id: Uuid) -> Result<ParsedFeed> {
    let feed = parser::parse(content)
        .map_err(|e| Error::FeedParse(e.to_string()))?;

    let title = feed.title.map(|t| t.content);
    let description = feed.description.map(|d| d.content);

    let site_url = feed.links.first().map(|l| l.href.clone());

    let icon_url = feed.icon.map(|i| i.uri)
        .or_else(|| feed.logo.map(|l| l.uri));

    let articles = feed.entries.into_iter().map(|entry| {
        let guid = entry.id;

        let url = entry.links.first().map(|l| l.href.clone());

        let title = entry.title
            .map(|t| t.content)
            .unwrap_or_else(|| "Untitled".to_string());

        let author = entry.authors.first().map(|a| a.name.clone());

        let content = entry.content
            .and_then(|c| c.body)
            .or_else(|| entry.summary.map(|s| s.content));

        let content_text = content.as_ref().map(|c| html_to_text(c));

        let published_at = entry.published
            .or(entry.updated)
            .map(|dt| DateTime::<Utc>::from(dt));

        NewArticle {
            feed_id,
            guid,
            url,
            title,
            author,
            content,
            content_text,
            published_at,
        }
    }).collect();

    Ok(ParsedFeed {
        title,
        description,
        site_url,
        icon_url,
        articles,
    })
}

/// Convert HTML content to plain text
fn html_to_text(html: &str) -> String {
    html2text::from_read(html.as_bytes(), 80)
        .unwrap_or_else(|_| html.to_string())
}
