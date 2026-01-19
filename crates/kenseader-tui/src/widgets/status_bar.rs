use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, Focus, Mode, ViewMode};
use crate::theme::GruvboxMaterial;

pub struct StatusBarWidget;

impl StatusBarWidget {
    pub fn render(frame: &mut Frame, area: Rect, app: &App) {
        // Check if in search mode - show search input prominently
        let is_search_mode = app.is_input_mode();

        // Check visual mode and selection count
        let is_visual = app.is_visual_mode();
        let selection_count = match app.focus {
            Focus::ArticleList | Focus::ArticleDetail => app.selected_articles.len(),
            Focus::Subscriptions => app.selected_feeds.len(),
        };

        // Read-mode prefix
        let read_mode_prefix = if app.read_mode { "[READ] " } else { "" };

        let mode_str: String = if app.is_refreshing {
            // Show animated spinner with SYNCING text
            format!("{}{} SYNCING", read_mode_prefix, app.current_spinner())
        } else {
            let base_mode = match &app.mode {
                Mode::Normal => {
                    if is_visual {
                        "VISUAL".to_string()
                    } else {
                        match app.view_mode {
                            ViewMode::All => "NORMAL".to_string(),
                            ViewMode::UnreadOnly => "UNREAD".to_string(),
                        }
                    }
                }
                Mode::SearchForward(_) => "SEARCH".to_string(),
                Mode::SearchBackward(_) => "SEARCH".to_string(),
                Mode::DeleteConfirm(_) => "CONFIRM".to_string(),
                Mode::BatchDeleteConfirm => "CONFIRM".to_string(),
                Mode::Help => "HELP".to_string(),
                Mode::ImageViewer(_) => "IMAGE".to_string(),
            };
            format!("{}{}", read_mode_prefix, base_mode)
        };

        // Selection info
        let selection_info = if selection_count > 0 {
            format!(" | Selected: {}", selection_count)
        } else {
            String::new()
        };

        let focus_str = match app.focus {
            Focus::Subscriptions => "Feeds",
            Focus::ArticleList => "Articles",
            Focus::ArticleDetail => "Detail",
        };

        let feed_count = app.feeds.len();
        let article_count = app.articles.len();

        let status_text = if is_search_mode {
            // Show search prompt with cursor and match count
            let search_char = match &app.mode {
                Mode::SearchForward(_) => "/",
                Mode::SearchBackward(_) => "?",
                _ => "/",
            };
            let match_info = if !app.search_query.is_empty() {
                let count = app.search_matches.len();
                if count > 0 {
                    format!(" ({} matches)", count)
                } else {
                    " (no matches)".to_string()
                }
            } else {
                String::new()
            };
            format!(" {}{}_{}", search_char, app.search_query, match_info)
        } else if let Some(msg) = &app.status_message {
            msg.clone()
        } else {
            format!(
                " {} | {} | Feeds: {} | Articles: {}{}",
                mode_str, focus_str, feed_count, article_count, selection_info
            )
        };

        let help_hint = " q:quit h/l:panels j/k:move /:search ?:help ";
        let padding_len = area.width.saturating_sub(
            status_text.len() as u16 + help_hint.len() as u16,
        ) as usize;

        let line = Line::from(vec![
            Span::styled(
                status_text,
                Style::default()
                    .fg(GruvboxMaterial::FG0)
                    .bg(GruvboxMaterial::BG2),
            ),
            Span::styled(
                " ".repeat(area.width as usize)
                    .chars()
                    .take(padding_len)
                    .collect::<String>(),
                Style::default().bg(GruvboxMaterial::BG2),
            ),
            Span::styled(
                help_hint,
                Style::default()
                    .fg(GruvboxMaterial::GREY2)
                    .bg(GruvboxMaterial::BG2),
            ),
        ]);

        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, area);
    }

}
