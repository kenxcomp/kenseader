use chrono::{DateTime, Utc};
use feed_rs::parser;
use uuid::Uuid;

use super::models::NewArticle;
use crate::{Error, Result};

/// Simple regex-like pattern matching for extracting image URLs from HTML
fn extract_first_image_url(html: &str) -> Option<String> {
    // Look for <img src="..."> patterns
    let html_lower = html.to_lowercase();

    // Find img tag
    if let Some(img_start) = html_lower.find("<img") {
        let remaining = &html[img_start..];

        // Find src attribute
        if let Some(src_start) = remaining.to_lowercase().find("src=") {
            let src_remaining = &remaining[src_start + 4..];

            // Handle both src="url" and src='url'
            let quote_char = src_remaining.chars().next()?;
            if quote_char == '"' || quote_char == '\'' {
                let url_start = 1;
                if let Some(url_end) = src_remaining[url_start..].find(quote_char) {
                    let url = &src_remaining[url_start..url_start + url_end];
                    // Filter out small images (likely icons/tracking pixels)
                    if !url.contains("1x1") && !url.contains("pixel") && !url.contains("tracking") {
                        return Some(url.to_string());
                    }
                }
            }
        }
    }

    // Also check for media:content or enclosure in feed entries (handled separately)
    None
}

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

        // Extract image URL from media content, enclosure, or HTML content
        let image_url = entry.media.first()
            .and_then(|m| m.thumbnails.first())
            .map(|t| t.image.uri.clone())
            .or_else(|| {
                // Check media content
                entry.media.first()
                    .and_then(|m| m.content.first())
                    .and_then(|c| c.url.as_ref())
                    .map(|u| u.to_string())
            })
            .or_else(|| {
                // Extract from HTML content
                content.as_ref().and_then(|c| extract_first_image_url(c))
            });

        NewArticle {
            feed_id,
            guid,
            url,
            title,
            author,
            content,
            content_text,
            published_at,
            image_url,
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
