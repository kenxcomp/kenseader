use ratatui::{
    layout::Rect,
    style::Style,
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

        // Check if we're searching
        let search_query = if !app.search_query.is_empty() {
            Some(app.search_query.to_lowercase())
        } else {
            None
        };

        let items: Vec<ListItem> = app
            .articles
            .iter()
            .enumerate()
            .map(|(i, article)| {
                let read_marker = if article.is_read { " " } else { "●" };
                let saved_marker = if article.is_saved { "★" } else { " " };

                let title = &article.title;

                let base_style = if i == app.selected_article && is_focused {
                    Style::default()
                        .fg(GruvboxMaterial::FG0)
                        .bg(GruvboxMaterial::SELECTION)
                } else if !article.is_read {
                    Style::default().fg(GruvboxMaterial::UNREAD)
                } else {
                    Style::default().fg(GruvboxMaterial::READ)
                };

                let marker_style = Style::default().fg(GruvboxMaterial::YELLOW);
                let saved_style = Style::default().fg(GruvboxMaterial::ORANGE);

                // Build title spans with search highlighting
                let title_spans = if let Some(ref query) = search_query {
                    Self::highlight_matches(title, query, base_style)
                } else {
                    vec![Span::styled(title.clone(), base_style)]
                };

                let mut spans = vec![
                    Span::styled(read_marker, marker_style),
                    Span::styled(saved_marker, saved_style),
                    Span::raw(" "),
                ];
                spans.extend(title_spans);

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(GruvboxMaterial::SELECTION),
            );

        let mut state = ListState::default();
        state.select(Some(app.selected_article));

        frame.render_stateful_widget(list, area, &mut state);
    }

    /// Highlight matching parts of a string with a different color
    fn highlight_matches<'a>(text: &'a str, query: &str, base_style: Style) -> Vec<Span<'a>> {
        let mut spans = Vec::new();
        let text_lower = text.to_lowercase();
        let mut last_end = 0;

        // Find all occurrences of the query in the text
        for (start, _) in text_lower.match_indices(query) {
            // Add non-matching part before this match
            if start > last_end {
                spans.push(Span::styled(
                    text[last_end..start].to_string(),
                    base_style,
                ));
            }

            // Add the matching part with highlight style
            let end = start + query.len();
            let highlight_style = base_style
                .fg(GruvboxMaterial::BG0)
                .bg(GruvboxMaterial::YELLOW);
            spans.push(Span::styled(
                text[start..end].to_string(),
                highlight_style,
            ));

            last_end = end;
        }

        // Add remaining non-matching part
        if last_end < text.len() {
            spans.push(Span::styled(
                text[last_end..].to_string(),
                base_style,
            ));
        }

        // If no matches found, return the whole text with base style
        if spans.is_empty() {
            spans.push(Span::styled(text.to_string(), base_style));
        }

        spans
    }
}
