use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::app::{App, Focus, ViewMode};
use crate::theme::GruvboxMaterial;

pub struct ArticleListWidget;

impl ArticleListWidget {
    pub fn render(frame: &mut Frame, area: Rect, app: &App) {
        let is_focused = app.focus == Focus::ArticleList;

        let border_style = if is_focused {
            Style::default().fg(GruvboxMaterial::ACCENT)
        } else {
            Style::default().fg(GruvboxMaterial::GREY0)
        };

        let mode_indicator = match app.view_mode {
            ViewMode::All => "",
            ViewMode::UnreadOnly => " [Unread]",
        };

        let title = format!(" Articles{} ", mode_indicator);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(Style::default().bg(GruvboxMaterial::BG0));

        let items: Vec<ListItem> = app
            .articles
            .iter()
            .enumerate()
            .map(|(i, article)| {
                let read_marker = if article.is_read { " " } else { "●" };
                let saved_marker = if article.is_saved { "★" } else { " " };

                let title = &article.title;

                let style = if i == app.selected_article && is_focused {
                    Style::default()
                        .fg(GruvboxMaterial::FG0)
                        .bg(GruvboxMaterial::SELECTION)
                        .add_modifier(Modifier::BOLD)
                } else if !article.is_read {
                    Style::default().fg(GruvboxMaterial::UNREAD)
                } else {
                    Style::default().fg(GruvboxMaterial::READ)
                };

                let marker_style = Style::default().fg(GruvboxMaterial::YELLOW);
                let saved_style = Style::default().fg(GruvboxMaterial::ORANGE);

                let line = Line::from(vec![
                    Span::styled(read_marker, marker_style),
                    Span::styled(saved_marker, saved_style),
                    Span::raw(" "),
                    Span::styled(title.clone(), style),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(GruvboxMaterial::SELECTION)
                    .add_modifier(Modifier::BOLD),
            );

        let mut state = ListState::default();
        state.select(Some(app.selected_article));

        frame.render_stateful_widget(list, area, &mut state);
    }
}
