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

        // Use visible feeds based on view mode
        let visible_feeds = app.visible_feeds();

        let block = Block::default()
            .title(" Subscriptions ")
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(Style::default().bg(GruvboxMaterial::BG0));

        // Calculate selected index in visible feeds
        let selected_visible_idx = app.actual_to_visible_feed_index(app.selected_feed);

        let items: Vec<ListItem> = visible_feeds
            .iter()
            .enumerate()
            .map(|(i, feed)| {
                // Get actual feed index to check selection
                let actual_idx = app.visible_to_actual_feed_index(i);
                let is_marked = actual_idx
                    .map(|idx| app.selected_feeds.contains(&idx))
                    .unwrap_or(false);

                // Selection marker (yazi-like)
                let select_marker = if is_marked { "✓" } else { " " };

                let unread = if feed.unread_count > 0 {
                    format!(" ({})", feed.unread_count)
                } else {
                    String::new()
                };

                let name = &feed.local_name;
                let is_cursor = selected_visible_idx == Some(i);

                // Determine style based on feed state
                // Priority: marked > cursor > error > unread > read
                let style = if is_marked {
                    Style::default()
                        .fg(GruvboxMaterial::FG0)
                        .bg(GruvboxMaterial::PURPLE)
                        .add_modifier(Modifier::BOLD)
                } else if is_cursor && is_focused {
                    Style::default()
                        .fg(GruvboxMaterial::FG0)
                        .bg(GruvboxMaterial::SELECTION)
                        .add_modifier(Modifier::BOLD)
                } else if feed.has_error() {
                    // Feeds with fetch errors are shown in red
                    Style::default().fg(GruvboxMaterial::ERROR)
                } else if feed.unread_count > 0 {
                    Style::default().fg(GruvboxMaterial::UNREAD)
                } else {
                    Style::default().fg(GruvboxMaterial::READ)
                };

                let select_style = if is_marked {
                    Style::default().fg(GruvboxMaterial::GREEN)
                } else {
                    Style::default().fg(GruvboxMaterial::GREY1)
                };

                // Add error indicator for feeds with errors
                let error_indicator = if feed.has_error() { " !" } else { "" };

                let line = Line::from(vec![
                    Span::styled(select_marker, select_style),
                    Span::styled(name.clone(), style),
                    Span::styled(error_indicator, Style::default().fg(GruvboxMaterial::ERROR)),
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
        state.select(selected_visible_idx);

        frame.render_stateful_widget(list, area, &mut state);
    }
}
