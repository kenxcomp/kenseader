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
        let mode_str = match &app.mode {
            Mode::Normal => match app.view_mode {
                ViewMode::All => "NORMAL",
                ViewMode::UnreadOnly => "UNREAD",
            },
            Mode::SearchForward(_) => "SEARCH /",
            Mode::SearchBackward(_) => "SEARCH ?",
            Mode::DeleteConfirm(_) => "DELETE?",
            Mode::Help => "HELP",
        };

        let focus_str = match app.focus {
            Focus::Subscriptions => "Feeds",
            Focus::ArticleList => "Articles",
            Focus::ArticleDetail => "Detail",
        };

        let feed_count = app.feeds.len();
        let article_count = app.articles.len();

        let status_text = if let Some(msg) = &app.status_message {
            msg.clone()
        } else {
            format!(
                " {} | {} | Feeds: {} | Articles: {}",
                mode_str, focus_str, feed_count, article_count
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
