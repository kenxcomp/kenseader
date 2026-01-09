use image::{DynamicImage, GenericImageView};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Focus, RichArticleState};
use crate::image_renderer::RenderBackend;
use crate::rich_content::{ContentElement, ImageState, ResizedImageCache};
use crate::theme::GruvboxMaterial;

/// Information about an image to render
struct ImageRenderInfo {
    /// URL of the image
    url: String,
    /// Y position in content lines (before scroll)
    content_y: u16,
    /// Height in terminal rows
    height: u16,
    /// Index in the image_urls list
    image_index: usize,
}

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

        // Check which backend to use for images
        let backend = app.image_renderer.backend();
        // Use external rendering for Ueberzug and Kitty (both need image_infos populated)
        let use_overlay = matches!(backend, RenderBackend::Ueberzug | RenderBackend::Kitty);

        // Track image positions for protocol rendering
        let mut image_infos: Vec<ImageRenderInfo> = Vec::new();

        let content = if let Some(article) = app.current_article().cloned() {
            // Build content with rich rendering if available
            if let Some(ref mut rich_state) = app.rich_state {
                // Recalculate heights if needed
                if rich_state.element_heights.is_empty()
                    || rich_state.viewport_height != inner_area.height
                {
                    rich_state.viewport_height = inner_area.height;
                    rich_state.calculate_heights(inner_area.width.saturating_sub(2));
                }
                Self::render_rich_content(
                    &article,
                    rich_state,
                    inner_area.width.saturating_sub(2),
                    show_author,
                    show_timestamps,
                    use_overlay,
                    &mut image_infos,
                )
            } else {
                Self::render_plain_content(&article, show_author, show_timestamps)
            }
        } else {
            Text::from(Line::from(Span::styled(
                "No article selected",
                Style::default().fg(GruvboxMaterial::GREY1),
            )))
        };

        // Don't use Paragraph's wrap - we handle wrapping manually to ensure
        // accurate line counting for image positioning
        let paragraph = Paragraph::new(content)
            .scroll((app.detail_scroll, 0));

        frame.render_widget(paragraph, inner_area);

        // Render images using the appropriate backend
        if !image_infos.is_empty() {
            match backend {
                RenderBackend::Ueberzug => {
                    // Render using Üeberzug++ overlay
                    Self::render_ueberzug_images(inner_area, app, &image_infos);
                }
                RenderBackend::Kitty => {
                    // Render using Kitty graphics protocol
                    Self::render_kitty_images(frame, inner_area, app, &image_infos);
                }
                RenderBackend::ITerm2 | RenderBackend::Sixel => {
                    // Render using halfblocks for native protocols (as fallback)
                    Self::render_protocol_images(frame, inner_area, app, &image_infos);
                }
                RenderBackend::Halfblocks => {
                    // Halfblocks are rendered inline via the Paragraph
                }
            }
        } else {
            // Clear all images when there are no images to display
            if backend == RenderBackend::Ueberzug || backend == RenderBackend::Kitty {
                app.image_renderer.clear_all();
            }
        }
    }

    /// Render images using Kitty graphics protocol
    fn render_kitty_images(
        frame: &mut Frame,
        area: Rect,
        app: &mut App,
        images: &[ImageRenderInfo],
    ) {
        let scroll = app.detail_scroll;
        let Some(ref mut rich_state) = app.rich_state else {
            app.image_renderer.clear_all();
            return;
        };

        // Collect visible images and their render info
        let mut visible_images: Vec<(String, image::DynamicImage, u16, u16, u16, u16)> = Vec::new();

        for img_info in images {
            // Calculate if image is visible in viewport
            let img_top_in_content = img_info.content_y;
            let img_bottom_in_content = img_info.content_y + img_info.height;

            // Skip if entirely above viewport
            if img_bottom_in_content <= scroll {
                continue;
            }
            // Skip if entirely below viewport
            if img_top_in_content >= scroll + area.height {
                continue;
            }

            // Get the cached image data
            let Some(cached) = rich_state.image_cache.get(&img_info.url) else {
                continue;
            };

            // Clone image to avoid borrow issues
            let image = cached.image.clone();

            // Calculate render position in viewport
            let render_y = if img_top_in_content >= scroll {
                area.y + (img_top_in_content - scroll)
            } else {
                area.y
            };

            // Calculate visible height
            let visible_top = scroll.saturating_sub(img_top_in_content);
            let available_height = area.height.saturating_sub(render_y - area.y);
            let render_height = (img_info.height - visible_top).min(available_height);

            if render_height == 0 {
                continue;
            }

            visible_images.push((
                img_info.url.clone(),
                image,
                area.x,
                render_y,
                area.width,
                render_height,
            ));
        }

        // Collect active URLs for cleanup
        let active_urls: Vec<String> = visible_images.iter().map(|(url, _, _, _, _, _)| url.clone()).collect();

        // Render each visible image using state-aware API
        // Collect failed images for fallback rendering
        let mut failed_images: Vec<(String, image::DynamicImage, u16, u16, u16, u16)> = Vec::new();

        if let Some(ref mut kitty) = app.image_renderer.kitty_renderer() {
            for (url, image, x, y, width, height) in visible_images {
                if let Err(e) = kitty.display_or_update(&url, &image, x, y, width, height) {
                    tracing::error!("Failed to display image via Kitty: {}", e);
                    // Collect for fallback rendering after releasing kitty borrow
                    failed_images.push((url, image, x, y, width, height));
                }
            }

            // Clean up images that are no longer visible
            let _ = kitty.end_frame(&active_urls);
        }

        // Fallback to halfblocks for failed images (now we can access rich_state again)
        if !failed_images.is_empty() {
            if let Some(ref mut rich_state) = app.rich_state {
                for (url, image, x, y, width, height) in failed_images {
                    let render_area = Rect { x, y, width, height };
                    Self::render_halfblocks_at_position(
                        frame,
                        render_area,
                        &image,
                        &mut rich_state.resized_cache,
                        &url,
                    );
                }
            }
        }
    }

    /// Render images using Üeberzug++ overlay
    fn render_ueberzug_images(area: Rect, app: &mut App, images: &[ImageRenderInfo]) {
        let scroll = app.detail_scroll;
        let Some(ref mut rich_state) = app.rich_state else {
            app.image_renderer.clear_all();
            return;
        };

        // Track which images are visible for cleanup
        let mut visible_images: std::collections::HashSet<String> = std::collections::HashSet::new();

        for img_info in images {
            // Calculate if image is visible in viewport
            let img_top_in_content = img_info.content_y;
            let img_bottom_in_content = img_info.content_y + img_info.height;

            // Skip if entirely above viewport
            if img_bottom_in_content <= scroll {
                continue;
            }
            // Skip if entirely below viewport
            if img_top_in_content >= scroll + area.height {
                continue;
            }

            // Get the cached image data with cache path
            let Some(cached) = rich_state.image_cache.get(&img_info.url) else {
                continue;
            };

            // We need a file path for Üeberzug++
            let Some(ref cache_path) = cached.cache_path else {
                continue;
            };

            // Calculate render position in viewport
            let render_y = if img_top_in_content >= scroll {
                area.y + (img_top_in_content - scroll)
            } else {
                area.y // Image starts above viewport, clip at top
            };

            // Calculate visible height
            let visible_top = scroll.saturating_sub(img_top_in_content);
            let available_height = area.height.saturating_sub(render_y - area.y);
            let render_height = (img_info.height - visible_top).min(available_height);

            if render_height == 0 {
                continue;
            }

            // Calculate actual screen position
            // For focused images, shrink by 1 for border
            let (x, y, width, height) = if rich_state.focused_image == Some(img_info.image_index) {
                (area.x + 1, render_y + 1, area.width.saturating_sub(2), render_height.saturating_sub(2))
            } else {
                (area.x, render_y, area.width, render_height)
            };

            // Use URL as identifier for Üeberzug
            let identifier = format!("img_{}", img_info.image_index);
            visible_images.insert(identifier.clone());

            // Render via Üeberzug++
            app.image_renderer.render(
                &identifier,
                cache_path,
                x,
                y,
                width,
                height,
            );
        }

        // Clear images that are no longer visible
        // Note: This is a simplified approach; a more robust implementation would track all active identifiers
    }

    /// Render images using halfblock characters with proper positioning
    fn render_protocol_images(
        frame: &mut Frame,
        area: Rect,
        app: &mut App,
        images: &[ImageRenderInfo],
    ) {
        let scroll = app.detail_scroll;
        let Some(ref mut rich_state) = app.rich_state else {
            return;
        };

        // First pass: collect render info and clone images to avoid borrow conflicts
        // (we need both image_cache for the image and resized_cache for caching)
        struct RenderItem {
            url: String,
            image: image::DynamicImage,
            render_area: Rect,
            is_focused: bool,
            image_area: Rect,
        }
        let mut render_items: Vec<RenderItem> = Vec::new();

        for img_info in images {
            // Calculate if image is visible in viewport
            let img_top_in_content = img_info.content_y;
            let img_bottom_in_content = img_info.content_y + img_info.height;

            // Skip if entirely above viewport
            if img_bottom_in_content <= scroll {
                continue;
            }
            // Skip if entirely below viewport
            if img_top_in_content >= scroll + area.height {
                continue;
            }

            // Get the cached image data
            let Some(cached) = rich_state.image_cache.get(&img_info.url) else {
                continue;
            };

            // Calculate render position in viewport
            let render_y = if img_top_in_content >= scroll {
                area.y + (img_top_in_content - scroll)
            } else {
                area.y // Image starts above viewport, clip at top
            };

            // Calculate visible height
            let visible_top = scroll.saturating_sub(img_top_in_content);
            let available_height = area.height.saturating_sub(render_y - area.y);
            let render_height = (img_info.height - visible_top).min(available_height);

            if render_height == 0 {
                continue;
            }

            let image_area = Rect {
                x: area.x,
                y: render_y,
                width: area.width,
                height: render_height,
            };

            let is_focused = rich_state.focused_image == Some(img_info.image_index);

            // Calculate render area (accounting for border if focused)
            let render_area = if is_focused {
                Rect {
                    x: image_area.x + 1,
                    y: image_area.y + 1,
                    width: image_area.width.saturating_sub(2),
                    height: image_area.height.saturating_sub(2),
                }
            } else {
                image_area
            };

            if render_area.width > 0 && render_area.height > 0 {
                render_items.push(RenderItem {
                    url: img_info.url.clone(),
                    image: cached.image.clone(),
                    render_area,
                    is_focused,
                    image_area,
                });
            }
        }

        // Second pass: render images using cached resized data
        for item in render_items {
            // Draw border for focused image
            if item.is_focused {
                let border = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(GruvboxMaterial::YELLOW));
                frame.render_widget(border, item.image_area);
            }

            // Use halfblock rendering with resize cache
            Self::render_halfblocks_at_position(
                frame,
                item.render_area,
                &item.image,
                &mut rich_state.resized_cache,
                &item.url,
            );
        }
    }

    /// Render image using halfblocks at a specific position
    /// Uses ResizedImageCache to avoid expensive resize/convert on every frame
    fn render_halfblocks_at_position(
        frame: &mut Frame,
        area: Rect,
        img: &DynamicImage,
        resized_cache: &mut ResizedImageCache,
        url: &str,
    ) {
        // Get pre-resized image from cache (or resize and cache if not present)
        let cached = resized_cache.get_or_resize(url, img, area.width, area.height);
        let rgba = &cached.rgba;
        let new_width = cached.pixel_width;
        let new_height = cached.pixel_height;

        // Calculate target dimensions for centering
        let target_width = area.width as u32;

        // Center the image
        let x_offset = target_width.saturating_sub(new_width) / 2;
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

    /// Render article using RichContent with inline images
    fn render_rich_content<'a>(
        article: &kenseader_core::feed::Article,
        rich_state: &mut RichArticleState,
        width: u16,
        show_author: bool,
        show_timestamps: bool,
        use_overlay: bool,
        image_infos: &mut Vec<ImageRenderInfo>,
    ) -> Text<'a> {
        let mut lines: Vec<Line<'a>> = Vec::new();
        let mut current_y: u16 = 0;
        let mut image_index: usize = 0;

        // Title
        lines.push(Line::from(Span::styled(
            article.title.clone(),
            Style::default()
                .fg(GruvboxMaterial::FG1)
                .add_modifier(Modifier::BOLD),
        )));
        current_y += 1;
        lines.push(Line::from(""));
        current_y += 1;

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
            current_y += 1;
            lines.push(Line::from(""));
            current_y += 1;
        }

        // Summary (if available) - render with box border
        if let Some(summary) = &article.summary {
            let summary_lines = render_summary_box(summary, width as usize);
            let summary_height = summary_lines.len() as u16;
            lines.extend(summary_lines);
            current_y += summary_height;
            lines.push(Line::from(""));
            current_y += 1;
        }

        // Render each content element with proper text wrapping
        let wrap_width = width as usize;

        for element in rich_state.content.elements.clone() {
            match element {
                ContentElement::Text(text) => {
                    // Wrap text to match Paragraph behavior
                    let wrapped = wrap_text_unicode(&text, wrap_width);
                    for line in wrapped {
                        lines.push(Line::from(Span::styled(
                            line,
                            Style::default().fg(GruvboxMaterial::FG0),
                        )));
                        current_y += 1;
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
                    // Wrap heading text
                    let wrapped = wrap_text_unicode(&text, wrap_width);
                    for line in wrapped {
                        lines.push(Line::from(Span::styled(line, style)));
                        current_y += 1;
                    }
                    lines.push(Line::from(""));
                    current_y += 1;
                }
                ContentElement::Image { ref url, ref alt } => {
                    let image_height = rich_state.image_height;

                    // Check if image is loaded and we're using overlay (Üeberzug/Kitty)
                    let is_loaded = rich_state.image_cache.is_ready(url);

                    if use_overlay && is_loaded {
                        // Record image position for overlay rendering
                        image_infos.push(ImageRenderInfo {
                            url: url.clone(),
                            content_y: current_y,
                            height: image_height,
                            image_index,
                        });

                        // Add placeholder empty lines (will be overlaid by image)
                        for _ in 0..image_height {
                            lines.push(Line::from(""));
                        }
                        current_y += image_height;

                        // Add empty line after image
                        lines.push(Line::from(""));
                        current_y += 1;
                    } else {
                        // Fallback: render using half-block characters or show status
                        let image_lines = Self::render_image_element(
                            url,
                            alt.as_deref(),
                            &mut rich_state.image_cache,
                            width as u32,
                            image_height as u32,
                            rich_state.focused_image == Some(image_index),
                        );
                        let line_count = image_lines.len() as u16;
                        lines.extend(image_lines);
                        current_y += line_count;
                    }
                    image_index += 1;
                }
                ContentElement::Quote(text) => {
                    // Wrap quote text (account for "| " prefix)
                    let quote_width = wrap_width.saturating_sub(2);
                    let wrapped = wrap_text_unicode(&text, quote_width);
                    for line in wrapped {
                        lines.push(Line::from(vec![
                            Span::styled("| ", Style::default().fg(GruvboxMaterial::GREY1)),
                            Span::styled(
                                line,
                                Style::default()
                                    .fg(GruvboxMaterial::FG0)
                                    .add_modifier(Modifier::ITALIC),
                            ),
                        ]));
                        current_y += 1;
                    }
                    lines.push(Line::from(""));
                    current_y += 1;
                }
                ContentElement::Code(text) => {
                    lines.push(Line::from(Span::styled(
                        "```",
                        Style::default().fg(GruvboxMaterial::GREY1),
                    )));
                    current_y += 1;
                    for line in text.lines() {
                        lines.push(Line::from(Span::styled(
                            line.to_string(),
                            Style::default()
                                .fg(GruvboxMaterial::GREEN)
                                .bg(GruvboxMaterial::BG1),
                        )));
                        current_y += 1;
                    }
                    lines.push(Line::from(Span::styled(
                        "```",
                        Style::default().fg(GruvboxMaterial::GREY1),
                    )));
                    current_y += 1;
                    lines.push(Line::from(""));
                    current_y += 1;
                }
                ContentElement::ListItem(text) => {
                    // Wrap list item text (account for "• " prefix)
                    let item_width = wrap_width.saturating_sub(2);
                    let wrapped = wrap_text_unicode(&text, item_width);
                    for (i, line) in wrapped.iter().enumerate() {
                        if i == 0 {
                            lines.push(Line::from(vec![
                                Span::styled("• ", Style::default().fg(GruvboxMaterial::AQUA)),
                                Span::styled(line.clone(), Style::default().fg(GruvboxMaterial::FG0)),
                            ]));
                        } else {
                            // Continuation lines indented
                            lines.push(Line::from(vec![
                                Span::styled("  ", Style::default()),
                                Span::styled(line.clone(), Style::default().fg(GruvboxMaterial::FG0)),
                            ]));
                        }
                        current_y += 1;
                    }
                }
                ContentElement::Separator => {
                    lines.push(Line::from(Span::styled(
                        "─".repeat(40.min(width as usize)),
                        Style::default().fg(GruvboxMaterial::GREY0),
                    )));
                    current_y += 1;
                }
                ContentElement::EmptyLine => {
                    lines.push(Line::from(""));
                    current_y += 1;
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

        // URL hint and image navigation hint
        if article.url.is_some() || !rich_state.content.image_urls.is_empty() {
            lines.push(Line::from(""));
            let mut hints = Vec::new();
            if article.url.is_some() {
                hints.push("'b' open in browser");
            }
            if !rich_state.content.image_urls.is_empty() {
                hints.push("Tab/Shift+Tab navigate images");
                hints.push("'o' open image");
                hints.push("Enter fullscreen");
            }
            lines.push(Line::from(Span::styled(
                hints.join(" | "),
                Style::default().fg(GruvboxMaterial::GREY1),
            )));
        }

        Text::from(lines)
    }

    /// Render a single image element (fallback halfblock rendering or status)
    fn render_image_element<'a>(
        url: &str,
        alt: Option<&str>,
        image_cache: &mut crate::rich_content::ArticleImageCache,
        width: u32,
        height: u32,
        is_focused: bool,
    ) -> Vec<Line<'a>> {
        let focus_style = if is_focused {
            Style::default().fg(GruvboxMaterial::YELLOW)
        } else {
            Style::default().fg(GruvboxMaterial::GREY1)
        };

        match image_cache.images.get_mut(url) {
            Some(ImageState::Loaded(cached)) => {
                // Render the image using half-block characters
                let mut lines = Self::render_image_to_lines(&cached.image, width, height);
                // Add focus indicator if focused
                if is_focused {
                    lines.insert(
                        0,
                        Line::from(Span::styled(
                            format!("┌{:─^width$}┐", " Image (focused) ", width = width as usize - 2),
                            Style::default().fg(GruvboxMaterial::YELLOW),
                        )),
                    );
                }
                lines
            }
            Some(ImageState::Loading) => {
                let prefix = if is_focused { "▶ " } else { "" };
                vec![
                    Line::from(Span::styled(
                        format!("{}[Loading image: {}]", prefix, truncate_url(url, 40)),
                        focus_style,
                    )),
                    Line::from(""),
                ]
            }
            Some(ImageState::Failed(err)) => {
                let prefix = if is_focused { "▶ " } else { "" };
                let display = if let Some(alt_text) = alt {
                    format!("{}[{}]", prefix, alt_text)
                } else {
                    format!("{}[Image failed: {}]", prefix, err)
                };
                vec![
                    Line::from(Span::styled(
                        display,
                        if is_focused {
                            Style::default().fg(GruvboxMaterial::YELLOW)
                        } else {
                            Style::default().fg(GruvboxMaterial::RED)
                        },
                    )),
                    Line::from(""),
                ]
            }
            None => {
                // Image not yet queued for loading
                let prefix = if is_focused { "▶ " } else { "" };
                vec![
                    Line::from(Span::styled(
                        format!("{}[Image: {}]", prefix, truncate_url(url, 50)),
                        focus_style,
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
