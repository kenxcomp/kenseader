use image::{DynamicImage, GenericImageView};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::image_renderer::RenderBackend;
use crate::theme::Theme;

pub struct ImageViewerWidget;

impl ImageViewerWidget {
    /// Render fullscreen image viewer
    pub fn render(frame: &mut Frame, area: Rect, app: &mut App, image_index: usize) {
        let theme = &app.theme;
        // Dark background
        let block = Block::default()
            .style(Style::default().bg(theme.bg0))
            .borders(Borders::NONE);
        frame.render_widget(block, area);

        let Some(ref mut rich_state) = app.rich_state else {
            Self::render_no_image(frame, area, "No article loaded", theme);
            return;
        };

        let image_count = rich_state.content.image_urls.len();
        if image_count == 0 {
            Self::render_no_image(frame, area, "No images in this article", theme);
            return;
        }

        let actual_index = image_index.min(image_count - 1);
        let Some(url) = rich_state.content.image_urls.get(actual_index) else {
            Self::render_no_image(frame, area, "Image not found", theme);
            return;
        };
        let url = url.clone();

        // Reserve space for status bar at bottom
        let status_height = 1;
        let image_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height.saturating_sub(status_height),
        };
        let status_area = Rect {
            x: area.x,
            y: area.y + image_area.height,
            width: area.width,
            height: status_height,
        };

        // Check if image is loaded
        if let Some(cached) = rich_state.image_cache.get(&url) {
            // Clone the image for rendering (to avoid borrow issues)
            let image = cached.image.clone();
            let cache_path = cached.cache_path.clone();

            // Check which backend to use
            let backend = app.image_renderer.backend();

            match backend {
                RenderBackend::Ueberzug => {
                    // Use Üeberzug++ for high-resolution fullscreen
                    if let Some(ref path) = cache_path {
                        let identifier = format!("fullscreen_{}", actual_index);
                        app.image_renderer.render(
                            &identifier,
                            path,
                            image_area.x,
                            image_area.y,
                            image_area.width,
                            image_area.height,
                        );
                    } else {
                        Self::render_fullscreen_halfblocks(frame, image_area, &image);
                    }
                }
                RenderBackend::Kitty => {
                    // Use Kitty graphics protocol for high-resolution display
                    if let Some(ref mut kitty) = app.image_renderer.kitty_renderer() {
                        // Use state-aware display to avoid flickering
                        let fullscreen_url = format!("fullscreen:{}", url);
                        if let Err(e) = kitty.display_or_update(
                            &fullscreen_url,
                            &image,
                            image_area.x,
                            image_area.y,
                            image_area.width,
                            image_area.height,
                        ) {
                            tracing::error!("Failed to display image via Kitty: {}", e);
                            Self::render_fullscreen_halfblocks(frame, image_area, &image);
                        }
                        // Clean up - only keep the fullscreen image
                        let _ = kitty.end_frame(&[fullscreen_url]);
                    } else {
                        Self::render_fullscreen_halfblocks(frame, image_area, &image);
                    }
                }
                _ => {
                    // Use halfblock rendering for reliable display
                    Self::render_fullscreen_halfblocks(frame, image_area, &image);
                }
            }
        } else if rich_state.image_cache.is_loading(&url) {
            Self::render_loading(frame, image_area, theme);
        } else {
            Self::render_no_image(frame, image_area, "Image not loaded", theme);
        }

        // Render status bar
        Self::render_status_bar(frame, status_area, actual_index + 1, image_count, theme);
    }

    /// Render status bar with navigation hints
    fn render_status_bar(frame: &mut Frame, area: Rect, current: usize, total: usize, theme: &Theme) {
        let status = Line::from(vec![
            Span::styled(
                format!(" Image {}/{} ", current, total),
                Style::default()
                    .fg(theme.bg0)
                    .bg(theme.yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled("n/→", Style::default().fg(theme.aqua)),
            Span::styled(" next ", Style::default().fg(theme.fg0)),
            Span::styled("p/←", Style::default().fg(theme.aqua)),
            Span::styled(" prev ", Style::default().fg(theme.fg0)),
            Span::styled("o", Style::default().fg(theme.aqua)),
            Span::styled(" open externally ", Style::default().fg(theme.fg0)),
            Span::styled("q/Esc", Style::default().fg(theme.aqua)),
            Span::styled(" close", Style::default().fg(theme.fg0)),
        ]);

        let paragraph = Paragraph::new(status).style(Style::default().bg(theme.bg1));
        frame.render_widget(paragraph, area);
    }

    /// Render loading message
    fn render_loading(frame: &mut Frame, area: Rect, theme: &Theme) {
        let message = Line::from(Span::styled(
            "Loading image...",
            Style::default()
                .fg(theme.yellow)
                .add_modifier(Modifier::BOLD),
        ));
        let paragraph = Paragraph::new(message)
            .style(Style::default().bg(theme.bg0))
            .alignment(ratatui::layout::Alignment::Center);

        // Center vertically
        let y_offset = area.height / 2;
        let centered_area = Rect {
            x: area.x,
            y: area.y + y_offset,
            width: area.width,
            height: 1,
        };
        frame.render_widget(paragraph, centered_area);
    }

    /// Render "no image" message
    fn render_no_image(frame: &mut Frame, area: Rect, message: &str, theme: &Theme) {
        let message = Line::from(Span::styled(
            message,
            Style::default()
                .fg(theme.grey1)
                .add_modifier(Modifier::ITALIC),
        ));
        let paragraph = Paragraph::new(message)
            .style(Style::default().bg(theme.bg0))
            .alignment(ratatui::layout::Alignment::Center);

        // Center vertically
        let y_offset = area.height / 2;
        let centered_area = Rect {
            x: area.x,
            y: area.y + y_offset,
            width: area.width,
            height: 1,
        };
        frame.render_widget(paragraph, centered_area);
    }

    /// Render image using fullscreen halfblock characters (fallback)
    fn render_fullscreen_halfblocks(frame: &mut Frame, area: Rect, img: &DynamicImage) {
        // Each character cell represents 2 vertical pixels
        let target_width = area.width as u32;
        let target_height = (area.height as u32) * 2; // Halfblocks = 2 pixels per row

        // Calculate aspect-ratio preserving dimensions
        let (img_width, img_height) = img.dimensions();
        let scale_w = target_width as f32 / img_width as f32;
        let scale_h = target_height as f32 / img_height as f32;
        let scale = scale_w.min(scale_h);

        let new_width = ((img_width as f32 * scale) as u32).max(1);
        let new_height = ((img_height as f32 * scale) as u32).max(1);

        // Resize image
        let resized = img.resize_exact(
            new_width,
            new_height,
            image::imageops::FilterType::Triangle, // Better quality for fullscreen
        );
        let rgba = resized.to_rgba8();

        // Center the image
        let x_offset = (target_width.saturating_sub(new_width)) / 2;
        let y_offset = (area.height as u32).saturating_sub(new_height / 2) / 2;

        // Render each row
        for row in 0..(new_height / 2) {
            let y = row * 2;
            let mut spans: Vec<Span> = Vec::with_capacity(target_width as usize);

            // Left padding
            if x_offset > 0 {
                spans.push(Span::raw(" ".repeat(x_offset as usize)));
            }

            // Image pixels
            for x in 0..new_width {
                let top_pixel = rgba.get_pixel(x, y);
                let bottom_pixel = if y + 1 < new_height {
                    rgba.get_pixel(x, y + 1)
                } else {
                    top_pixel
                };

                let top_color = Color::Rgb(top_pixel[0], top_pixel[1], top_pixel[2]);
                let bottom_color = Color::Rgb(bottom_pixel[0], bottom_pixel[1], bottom_pixel[2]);

                spans.push(Span::styled(
                    "▀",
                    Style::default().fg(top_color).bg(bottom_color),
                ));
            }

            let line = Line::from(spans);
            let line_area = Rect {
                x: area.x,
                y: area.y + y_offset as u16 + row as u16,
                width: area.width,
                height: 1,
            };

            if line_area.y < area.y + area.height {
                frame.render_widget(Paragraph::new(line), line_area);
            }
        }
    }
}
