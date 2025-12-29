use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

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
            .block(block)
            .wrap(Wrap { trim: true })
            .scroll((app.detail_scroll, 0));

        frame.render_widget(paragraph, area);
    }
}
