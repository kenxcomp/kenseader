use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use ratatui_image::{picker::Picker, StatefulImage, Resize};

use crate::app::{App, Focus};
use crate::theme::GruvboxMaterial;

pub struct ArticleDetailWidget;

impl ArticleDetailWidget {
    pub fn render(frame: &mut Frame, area: Rect, app: &App) {
        let is_focused = app.focus == Focus::ArticleDetail;

        let border_style = if is_focused {
            Style::default().fg(GruvboxMaterial::ACCENT)
        } else {
            Style::default().fg(GruvboxMaterial::GREY0)
        };

        let block = Block::default()
            .title(" Article ")
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(Style::default().bg(GruvboxMaterial::BG0));

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        // Check if we have an image to render
        let has_image = app.image_cache.as_ref()
            .map(|c| c.data.is_some())
            .unwrap_or(false);

        // Split the area if we have an image
        let (image_area, text_area) = if has_image && app.config.ui.image_preview {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(12), // Image height
                    Constraint::Min(1),     // Text content
                ])
                .split(inner_area);
            (Some(chunks[0]), chunks[1])
        } else {
            (None, inner_area)
        };

        // Render image if available
        if let (Some(img_area), Some(cache)) = (image_area, &app.image_cache) {
            if let Some(ref img_data) = cache.data {
                // Try to create a picker and render the image
                if let Ok(mut picker) = Picker::from_query_stdio() {
                    let protocol = picker.new_resize_protocol(img_data.clone());
                    let image_widget = StatefulImage::new(None).resize(Resize::Fit(None));
                    // For StatefulImage, we need a mutable protocol
                    // This is a simplified approach - in production you'd want to cache the protocol
                    let mut proto = protocol;
                    frame.render_stateful_widget(image_widget, img_area, &mut proto);
                }
            } else if cache.loading {
                let loading = Paragraph::new(Line::from(Span::styled(
                    "Loading image...",
                    Style::default().fg(GruvboxMaterial::GREY1),
                )));
                frame.render_widget(loading, img_area);
            }
        }

        // Render text content
        let content = if let Some(article) = app.current_article() {
            let mut lines = Vec::new();

            // Title
            lines.push(Line::from(Span::styled(
                &article.title,
                Style::default()
                    .fg(GruvboxMaterial::FG1)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));

            // Metadata
            let mut meta_spans = Vec::new();
            if let Some(author) = &article.author {
                meta_spans.push(Span::styled(
                    format!("By {} ", author),
                    Style::default().fg(GruvboxMaterial::GREY2),
                ));
            }
            if let Some(date) = &article.published_at {
                meta_spans.push(Span::styled(
                    format!("• {}", date.format("%Y-%m-%d %H:%M")),
                    Style::default().fg(GruvboxMaterial::GREY1),
                ));
            }
            if !meta_spans.is_empty() {
                lines.push(Line::from(meta_spans));
                lines.push(Line::from(""));
            }

            // Image indicator (if has image but not rendered)
            if article.image_url.is_some() && !has_image {
                lines.push(Line::from(Span::styled(
                    "[Image available]",
                    Style::default().fg(GruvboxMaterial::AQUA),
                )));
                lines.push(Line::from(""));
            }

            // Summary (if available)
            if let Some(summary) = &article.summary {
                lines.push(Line::from(Span::styled(
                    "Summary:",
                    Style::default()
                        .fg(GruvboxMaterial::AQUA)
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(Span::styled(
                    summary.clone(),
                    Style::default().fg(GruvboxMaterial::FG0),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "─".repeat(40),
                    Style::default().fg(GruvboxMaterial::GREY0),
                )));
                lines.push(Line::from(""));
            }

            // Content
            if let Some(content_text) = &article.content_text {
                for line in content_text.lines() {
                    lines.push(Line::from(Span::styled(
                        line.to_string(),
                        Style::default().fg(GruvboxMaterial::FG0),
                    )));
                }
            }

            // Tags
            if !article.tags.is_empty() {
                lines.push(Line::from(""));
                let tags_str = article.tags.join(" • ");
                lines.push(Line::from(Span::styled(
                    format!("Tags: {}", tags_str),
                    Style::default().fg(GruvboxMaterial::PURPLE),
                )));
            }

            // URL hint
            if article.url.is_some() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Press 'b' to open in browser",
                    Style::default().fg(GruvboxMaterial::GREY1),
                )));
            }

            Text::from(lines)
        } else {
            Text::from(Line::from(Span::styled(
                "No article selected",
                Style::default().fg(GruvboxMaterial::GREY1),
            )))
        };

        let paragraph = Paragraph::new(content)
            .wrap(Wrap { trim: true })
            .scroll((app.detail_scroll, 0));

        frame.render_widget(paragraph, text_area);
    }
}
