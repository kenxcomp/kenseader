use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::app::{App, Focus};
use crate::theme::GruvboxMaterial;

pub struct SubscriptionsWidget;

impl SubscriptionsWidget {
    pub fn render(frame: &mut Frame, area: Rect, app: &App) {
        let is_focused = app.focus == Focus::Subscriptions;

        let border_style = if is_focused {
            Style::default().fg(GruvboxMaterial::ACCENT)
        } else {
            Style::default().fg(GruvboxMaterial::GREY0)
        };

        let block = Block::default()
            .title(" Subscriptions ")
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(Style::default().bg(GruvboxMaterial::BG0));

        let items: Vec<ListItem> = app
            .feeds
            .iter()
            .enumerate()
            .map(|(i, feed)| {
                let unread = if feed.unread_count > 0 {
                    format!(" ({})", feed.unread_count)
                } else {
                    String::new()
                };

                let name = &feed.local_name;
                let style = if i == app.selected_feed && is_focused {
                    Style::default()
                        .fg(GruvboxMaterial::FG0)
                        .bg(GruvboxMaterial::SELECTION)
                        .add_modifier(Modifier::BOLD)
                } else if feed.unread_count > 0 {
                    Style::default().fg(GruvboxMaterial::UNREAD)
                } else {
                    Style::default().fg(GruvboxMaterial::READ)
                };

                let line = Line::from(vec![
                    Span::styled(name.clone(), style),
                    Span::styled(unread, Style::default().fg(GruvboxMaterial::YELLOW)),
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
        state.select(Some(app.selected_feed));

        frame.render_stateful_widget(list, area, &mut state);
    }
}
