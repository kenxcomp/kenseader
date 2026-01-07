use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::theme::GruvboxMaterial;

pub struct PopupWidget;

impl PopupWidget {
    /// Render a confirmation popup dialog
    pub fn render_confirm(
        frame: &mut Frame,
        title: &str,
        message: &str,
    ) {
        let area = frame.area();

        // Calculate popup size - centered, reasonable width
        let popup_width = 50u16.min(area.width.saturating_sub(4));
        let popup_height = 7u16.min(area.height.saturating_sub(2));

        let popup_area = centered_rect(popup_width, popup_height, area);

        // Clear the background area
        frame.render_widget(Clear, popup_area);

        // Create the popup block with border
        let block = Block::default()
            .title(format!(" {} ", title))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(GruvboxMaterial::ERROR))
            .style(Style::default().bg(GruvboxMaterial::BG1));

        // Inner layout for content
        let inner_area = block.inner(popup_area);

        frame.render_widget(block, popup_area);

        // Split inner area into message and hint sections
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Message
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Hint
            ])
            .split(inner_area);

        // Render message
        let message_paragraph = Paragraph::new(Line::from(vec![
            Span::styled(
                message,
                Style::default()
                    .fg(GruvboxMaterial::FG0)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .alignment(Alignment::Center);

        frame.render_widget(message_paragraph, chunks[0]);

        // Render hint (y/n options)
        let hint_paragraph = Paragraph::new(Line::from(vec![
            Span::styled("[", Style::default().fg(GruvboxMaterial::GREY1)),
            Span::styled(
                "y",
                Style::default()
                    .fg(GruvboxMaterial::GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("]es  [", Style::default().fg(GruvboxMaterial::GREY1)),
            Span::styled(
                "n",
                Style::default()
                    .fg(GruvboxMaterial::ERROR)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("]o", Style::default().fg(GruvboxMaterial::GREY1)),
        ]))
        .alignment(Alignment::Center);

        frame.render_widget(hint_paragraph, chunks[2]);
    }

    /// Render a delete confirmation popup for a single feed
    pub fn render_delete_confirm(frame: &mut Frame, feed_name: &str) {
        let message = format!("Delete feed \"{}\"?", truncate_str(feed_name, 30));
        Self::render_confirm(frame, "Confirm Delete", &message);
    }

    /// Render a batch delete confirmation popup
    pub fn render_batch_delete_confirm(frame: &mut Frame, count: usize) {
        let message = format!("Delete {} selected feeds?", count);
        Self::render_confirm(frame, "Confirm Batch Delete", &message);
    }
}

/// Helper function to create a centered rect
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

/// Truncate a string to max length with ellipsis
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}
