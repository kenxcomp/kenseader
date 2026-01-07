use image::{DynamicImage, GenericImageView};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Focus, RichArticleState};
use crate::rich_content::{ContentElement, ImageState};
use crate::theme::GruvboxMaterial;

pub struct ArticleDetailWidget;

impl ArticleDetailWidget {
    pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
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

        // Get UI config options
        let show_author = app.config.ui.show_author;
        let show_timestamps = app.config.ui.show_timestamps;

        let content = if let Some(article) = app.current_article().cloned() {
            // Build content with rich rendering if available
            if let Some(ref mut rich_state) = app.rich_state {
                // Recalculate heights if needed
                if rich_state.element_heights.is_empty() || rich_state.viewport_height != inner_area.height {
                    rich_state.viewport_height = inner_area.height;
                    rich_state.calculate_heights(inner_area.width.saturating_sub(2));
                }
                Self::render_rich_content(&article, rich_state, inner_area.width.saturating_sub(2), show_author, show_timestamps)
            } else {
                Self::render_plain_content(&article, show_author, show_timestamps)
            }
        } else {
            Text::from(Line::from(Span::styled(
                "No article selected",
                Style::default().fg(GruvboxMaterial::GREY1),
            )))
        };

        let paragraph = Paragraph::new(content)
            .wrap(Wrap { trim: true })
            .scroll((app.detail_scroll, 0));

        frame.render_widget(paragraph, inner_area);
    }

    /// Render article using RichContent with inline images
    fn render_rich_content<'a>(
        article: &kenseader_core::feed::Article,
        rich_state: &mut RichArticleState,
        width: u16,
        show_author: bool,
        show_timestamps: bool,
    ) -> Text<'a> {
        let mut lines: Vec<Line<'a>> = Vec::new();

        // Title
        lines.push(Line::from(Span::styled(
            article.title.clone(),
            Style::default()
                .fg(GruvboxMaterial::FG1)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        // Metadata (controlled by show_author and show_timestamps config)
        let mut meta_spans = Vec::new();
        if show_author {
            if let Some(author) = &article.author {
                meta_spans.push(Span::styled(
                    format!("By {} ", author),
                    Style::default().fg(GruvboxMaterial::GREY2),
                ));
            }
        }
        if show_timestamps {
            if let Some(date) = &article.published_at {
                let separator = if meta_spans.is_empty() { "" } else { "| " };
                meta_spans.push(Span::styled(
                    format!("{}{}", separator, date.format("%Y-%m-%d %H:%M")),
                    Style::default().fg(GruvboxMaterial::GREY1),
                ));
            }
        }
        if !meta_spans.is_empty() {
            lines.push(Line::from(meta_spans));
            lines.push(Line::from(""));
        }

        // Summary (if available) - render with box border
        if let Some(summary) = &article.summary {
            let summary_lines = render_summary_box(summary, width as usize);
            lines.extend(summary_lines);
            lines.push(Line::from(""));
        }

        // Render each content element
        for element in rich_state.content.elements.clone() {
            match element {
                ContentElement::Text(text) => {
                    for line in text.lines() {
                        lines.push(Line::from(Span::styled(
                            line.to_string(),
                            Style::default().fg(GruvboxMaterial::FG0),
                        )));
                    }
                }
                ContentElement::Heading(level, text) => {
                    let style = match level {
                        1 => Style::default()
                            .fg(GruvboxMaterial::ORANGE)
                            .add_modifier(Modifier::BOLD),
                        2 => Style::default()
                            .fg(GruvboxMaterial::YELLOW)
                            .add_modifier(Modifier::BOLD),
                        _ => Style::default()
                            .fg(GruvboxMaterial::AQUA)
                            .add_modifier(Modifier::BOLD),
                    };
                    lines.push(Line::from(Span::styled(text, style)));
                    lines.push(Line::from(""));
                }
                ContentElement::Image { url, alt } => {
                    // Try to render the image
                    let image_lines = Self::render_image_element(
                        &url,
                        alt.as_deref(),
                        &mut rich_state.image_cache,
                        width as u32,
                        rich_state.image_height as u32,
                    );
                    lines.extend(image_lines);
                }
                ContentElement::Quote(text) => {
                    for line in text.lines() {
                        lines.push(Line::from(vec![
                            Span::styled("| ", Style::default().fg(GruvboxMaterial::GREY1)),
                            Span::styled(
                                line.to_string(),
                                Style::default()
                                    .fg(GruvboxMaterial::FG0)
                                    .add_modifier(Modifier::ITALIC),
                            ),
                        ]));
                    }
                    lines.push(Line::from(""));
                }
                ContentElement::Code(text) => {
                    lines.push(Line::from(Span::styled(
                        "```",
                        Style::default().fg(GruvboxMaterial::GREY1),
                    )));
                    for line in text.lines() {
                        lines.push(Line::from(Span::styled(
                            line.to_string(),
                            Style::default()
                                .fg(GruvboxMaterial::GREEN)
                                .bg(GruvboxMaterial::BG1),
                        )));
                    }
                    lines.push(Line::from(Span::styled(
                        "```",
                        Style::default().fg(GruvboxMaterial::GREY1),
                    )));
                    lines.push(Line::from(""));
                }
                ContentElement::ListItem(text) => {
                    lines.push(Line::from(vec![
                        Span::styled("• ", Style::default().fg(GruvboxMaterial::AQUA)),
                        Span::styled(text, Style::default().fg(GruvboxMaterial::FG0)),
                    ]));
                }
                ContentElement::Separator => {
                    lines.push(Line::from(Span::styled(
                        "─".repeat(40.min(width as usize)),
                        Style::default().fg(GruvboxMaterial::GREY0),
                    )));
                }
                ContentElement::EmptyLine => {
                    lines.push(Line::from(""));
                }
            }
        }

        // Tags
        if !article.tags.is_empty() {
            lines.push(Line::from(""));
            let tags_str = article.tags.join(" | ");
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
    }

    /// Render a single image element
    fn render_image_element<'a>(
        url: &str,
        alt: Option<&str>,
        image_cache: &mut crate::rich_content::ArticleImageCache,
        width: u32,
        height: u32,
    ) -> Vec<Line<'a>> {
        match image_cache.images.get_mut(url) {
            Some(ImageState::Loaded(cached)) => {
                // Render the image using half-block characters
                Self::render_image_to_lines(&cached.image, width, height)
            }
            Some(ImageState::Loading) => {
                vec![
                    Line::from(Span::styled(
                        format!("[Loading image: {}]", truncate_url(url, 40)),
                        Style::default().fg(GruvboxMaterial::GREY1),
                    )),
                    Line::from(""),
                ]
            }
            Some(ImageState::Failed(err)) => {
                let display = if let Some(alt_text) = alt {
                    format!("[{}]", alt_text)
                } else {
                    format!("[Image failed: {}]", err)
                };
                vec![
                    Line::from(Span::styled(
                        display,
                        Style::default().fg(GruvboxMaterial::RED),
                    )),
                    Line::from(""),
                ]
            }
            None => {
                // Image not yet queued for loading
                vec![
                    Line::from(Span::styled(
                        format!("[Image: {}]", truncate_url(url, 50)),
                        Style::default().fg(GruvboxMaterial::GREY1),
                    )),
                    Line::from(""),
                ]
            }
        }
    }

    /// Fallback: render plain text content (when RichArticleState is not available)
    fn render_plain_content<'a>(
        article: &kenseader_core::feed::Article,
        show_author: bool,
        show_timestamps: bool,
    ) -> Text<'a> {
        let mut lines = Vec::new();

        // Title
        lines.push(Line::from(Span::styled(
            article.title.clone(),
            Style::default()
                .fg(GruvboxMaterial::FG1)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        // Metadata (controlled by show_author and show_timestamps config)
        let mut meta_spans = Vec::new();
        if show_author {
            if let Some(author) = &article.author {
                meta_spans.push(Span::styled(
                    format!("By {} ", author),
                    Style::default().fg(GruvboxMaterial::GREY2),
                ));
            }
        }
        if show_timestamps {
            if let Some(date) = &article.published_at {
                let separator = if meta_spans.is_empty() { "" } else { "| " };
                meta_spans.push(Span::styled(
                    format!("{}{}", separator, date.format("%Y-%m-%d %H:%M")),
                    Style::default().fg(GruvboxMaterial::GREY1),
                ));
            }
        }
        if !meta_spans.is_empty() {
            lines.push(Line::from(meta_spans));
            lines.push(Line::from(""));
        }

        // Summary (if available) - render with box border
        if let Some(summary) = &article.summary {
            let summary_lines = render_summary_box(summary, 70); // Fixed width for plain content
            lines.extend(summary_lines);
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
            let tags_str = article.tags.join(" | ");
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
    }

    /// Render an image to terminal lines using half-block characters
    /// Uses ▀ (upper half block) with fg=top pixel, bg=bottom pixel
    /// This works in any terminal with true color support
    fn render_image_to_lines<'a>(
        img: &DynamicImage,
        target_width: u32,
        target_height: u32,
    ) -> Vec<Line<'a>> {
        // Each character cell represents 2 vertical pixels
        let char_height = target_height * 2;

        // Calculate aspect-ratio preserving dimensions
        let (img_width, img_height) = img.dimensions();
        let scale_w = target_width as f32 / img_width as f32;
        let scale_h = char_height as f32 / img_height as f32;
        let scale = scale_w.min(scale_h);

        let new_width = ((img_width as f32 * scale) as u32).max(1);
        let new_height = ((img_height as f32 * scale) as u32).max(1);

        // Resize image using fast nearest-neighbor for performance
        let resized = img.resize_exact(
            new_width,
            new_height,
            image::imageops::FilterType::Nearest,
        );
        let rgba = resized.to_rgba8();

        // Center the image horizontally
        let x_offset = (target_width.saturating_sub(new_width)) / 2;
        let padding = " ".repeat(x_offset as usize);

        let mut lines = Vec::with_capacity((new_height / 2 + 1) as usize);

        // Process 2 rows at a time (top pixel = fg, bottom pixel = bg)
        for y in (0..new_height).step_by(2) {
            let mut spans: Vec<Span<'a>> = Vec::with_capacity(new_width as usize + 1);

            // Add left padding for centering
            if x_offset > 0 {
                spans.push(Span::raw(padding.clone()));
            }

            for x in 0..new_width {
                let top_pixel = rgba.get_pixel(x, y);
                let bottom_pixel = if y + 1 < new_height {
                    rgba.get_pixel(x, y + 1)
                } else {
                    top_pixel // Use top pixel if no bottom pixel
                };

                let top_color = Color::Rgb(top_pixel[0], top_pixel[1], top_pixel[2]);
                let bottom_color = Color::Rgb(bottom_pixel[0], bottom_pixel[1], bottom_pixel[2]);

                // Use upper half block: ▀
                // Foreground = top pixel, Background = bottom pixel
                spans.push(Span::styled(
                    "▀",
                    Style::default().fg(top_color).bg(bottom_color),
                ));
            }

            lines.push(Line::from(spans));
        }

        // Add empty line after image
        lines.push(Line::from(""));

        lines
    }
}

/// Truncate URL for display (UTF-8 safe)
fn truncate_url(url: &str, max_len: usize) -> String {
    if url.chars().count() <= max_len {
        url.to_string()
    } else {
        let truncated: String = url.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

/// Render summary text in a styled box with proper unicode width handling
fn render_summary_box<'a>(summary: &str, max_width: usize) -> Vec<Line<'a>> {
    let border_color = GruvboxMaterial::AQUA;
    let title = " AI Summary ";

    // Box inner width (excluding border characters "│ " and " │")
    let inner_width = max_width.saturating_sub(4).max(20);
    // Total box width including borders
    let box_width = inner_width + 2; // "│" + content + "│"

    let mut lines: Vec<Line<'a>> = Vec::new();

    // Top border: ╭─── AI Summary ───╮
    let title_width = title.width();
    let remaining = box_width.saturating_sub(title_width);
    let left_dashes = remaining / 2;
    let right_dashes = remaining - left_dashes;
    lines.push(Line::from(vec![
        Span::styled("╭", Style::default().fg(border_color)),
        Span::styled("─".repeat(left_dashes), Style::default().fg(border_color)),
        Span::styled(title, Style::default().fg(GruvboxMaterial::YELLOW).add_modifier(Modifier::BOLD)),
        Span::styled("─".repeat(right_dashes), Style::default().fg(border_color)),
        Span::styled("╮", Style::default().fg(border_color)),
    ]));

    // Wrap and render summary content
    let wrapped_lines = wrap_text_unicode(summary, inner_width);
    for content in wrapped_lines {
        let content_width = content.width();
        let padding = inner_width.saturating_sub(content_width);
        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(border_color)),
            Span::styled(content, Style::default().fg(GruvboxMaterial::FG0)),
            Span::styled(" ".repeat(padding), Style::default().fg(GruvboxMaterial::FG0)),
            Span::styled(" │", Style::default().fg(border_color)),
        ]));
    }

    // Bottom border: ╰────────────────╯
    lines.push(Line::from(vec![
        Span::styled("╰", Style::default().fg(border_color)),
        Span::styled("─".repeat(box_width), Style::default().fg(border_color)),
        Span::styled("╯", Style::default().fg(border_color)),
    ]));

    lines
}

/// Wrap text respecting unicode character widths (CJK = 2 columns)
fn wrap_text_unicode(text: &str, max_width: usize) -> Vec<String> {
    let mut result = Vec::new();

    for paragraph in text.lines() {
        if paragraph.is_empty() {
            result.push(String::new());
            continue;
        }

        let mut current_line = String::new();
        let mut current_width = 0;

        for ch in paragraph.chars() {
            let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);

            if current_width + ch_width > max_width {
                // Start new line
                if !current_line.is_empty() {
                    result.push(current_line);
                }
                current_line = String::new();
                current_width = 0;
            }

            current_line.push(ch);
            current_width += ch_width;
        }

        if !current_line.is_empty() {
            result.push(current_line);
        }
    }

    // Ensure at least one line
    if result.is_empty() {
        result.push(String::new());
    }

    result
}
