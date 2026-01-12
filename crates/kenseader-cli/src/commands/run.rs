use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use uuid::Uuid;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use tokio::sync::mpsc;

use kenseader_core::{
    ipc::DaemonClient,
    AppConfig,
};
use kenseader_tui::{
    app::{App, Focus, Mode, RichArticleState, ViewMode},
    event::{AppEvent, EventHandler, ImageLoadResult, RefreshResult},
    input::{handle_key_event, Action},
    rich_content::{download_image, FocusableItem},
    widgets::{
        ArticleDetailWidget, ArticleListWidget, ImageViewerWidget, PopupWidget, StatusBarWidget,
        SubscriptionsWidget,
    },
};

pub async fn run(config: Arc<AppConfig>) -> Result<()> {
    // Create daemon client and check if daemon is running
    let client = Arc::new(DaemonClient::new(config.socket_path()));

    if !client.ping().await? {
        return Err(anyhow!(
            "Daemon is not running.\nPlease start the daemon first with:\n  kenseader daemon start"
        ));
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(client.clone(), config.clone());

    // Load initial data
    load_feeds(&mut app).await?;

    // Create event handler
    let event_handler = EventHandler::new(config.ui.tick_rate_ms);

    // Get data directory for disk cache
    let data_dir = dirs::data_dir()
        .map(|d| d.join("kenseader"))
        .or_else(|| dirs::home_dir().map(|d| d.join(".kenseader")));

    // Initialize preload cache with disk cache support
    app.init_preload_cache(data_dir.as_ref());

    // Initialize rich state for the first article
    init_rich_article_state(&mut app, data_dir.as_ref());

    // Create channel for async image loading results
    let (img_tx, mut img_rx) = mpsc::unbounded_channel::<ImageLoadResult>();

    // Create channel for async refresh results
    let (refresh_tx, mut refresh_rx) = mpsc::unbounded_channel::<RefreshResult>();

    // Main loop
    loop {
        // Process any completed image loads (non-blocking)
        while let Ok(result) = img_rx.try_recv() {
            handle_image_result(&mut app, result);
        }

        // Process any completed refresh operations (non-blocking)
        while let Ok(result) = refresh_rx.try_recv() {
            handle_refresh_result(&mut app, result, &client, data_dir.as_ref()).await?;
        }

        // Preload images for nearby articles (when in article list view)
        if app.focus == Focus::ArticleList && app.config.ui.image_preview {
            process_preload(&mut app, &img_tx, data_dir.as_ref());
        }

        // Check if we need to load more images (visible-first strategy)
        if let Some(ref mut rich_state) = app.rich_state {
            let urls_to_load = rich_state.get_urls_needing_load(
                app.detail_scroll,
                rich_state.viewport_height,
            );
            for url in urls_to_load {
                // Mark as loading
                rich_state.image_cache.start_loading(&url);
                // Spawn async download task
                spawn_image_load(url, img_tx.clone(), data_dir.clone(), &rich_state.image_cache);
            }
        }

        // Draw UI
        terminal.draw(|frame| {
            let size = frame.area();
            // Update viewport height for adaptive scrolling
            app.viewport_height = size.height;

            // Check if we're in fullscreen image viewer mode
            if let Mode::ImageViewer(image_index) = app.mode {
                ImageViewerWidget::render(frame, size, &mut app, image_index);
                return;
            }

            // Main layout: content + status bar
            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(size);

            // Three-column layout with 1:4:5 ratio
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 10), // Subscriptions
                    Constraint::Ratio(4, 10), // Article list
                    Constraint::Ratio(5, 10), // Article detail
                ])
                .split(main_layout[0]);

            // Render widgets
            SubscriptionsWidget::render(frame, columns[0], &app);
            ArticleListWidget::render(frame, columns[1], &app);
            ArticleDetailWidget::render(frame, columns[2], &mut app);
            StatusBarWidget::render(frame, main_layout[1], &app);

            // Render popup dialogs on top (if in confirmation mode)
            match &app.mode {
                Mode::DeleteConfirm(_) => {
                    let feed_name = app
                        .current_feed()
                        .map(|f| f.local_name.as_str())
                        .unwrap_or("Unknown");
                    PopupWidget::render_delete_confirm(frame, feed_name);
                }
                Mode::BatchDeleteConfirm => {
                    let count = app.selected_feeds.len();
                    PopupWidget::render_batch_delete_confirm(frame, count);
                }
                _ => {}
            }
        })?;

        // Handle events
        if let Some(event) = event_handler.next()? {
            match event {
                AppEvent::Key(key) => {
                    let action = handle_key_event(key, &app);
                    handle_action(&mut app, action, &client, data_dir.as_ref(), refresh_tx.clone()).await?;
                }
                AppEvent::Resize(_, _) => {
                    // Recalculate heights on resize
                    if let Some(ref mut rich_state) = app.rich_state {
                        rich_state.element_heights.clear();
                    }
                }
                AppEvent::Tick => {
                    // Tick spinner animation for loading indicator
                    app.tick_spinner();
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

/// Handle completed image load result
fn handle_image_result(app: &mut App, result: ImageLoadResult) {
    match result {
        ImageLoadResult::Success {
            url,
            image,
            bytes,
            cache_path,
        } => {
            // Always update preload cache for future use
            app.preload_cache.set_loaded(&url, image.clone(), cache_path.clone());

            // Update current article's cache if it exists
            if let Some(ref mut rich_state) = app.rich_state {
                // Save to disk cache if we have bytes
                if !bytes.is_empty() {
                    rich_state.image_cache.save_to_disk(&url, &bytes);
                }
                // Store in memory cache with cache path
                rich_state.image_cache.set_loaded(&url, image, cache_path);
            }
        }
        ImageLoadResult::Failure { url, error } => {
            // Update preload cache
            app.preload_cache.set_failed(&url, error.clone());

            // Update current article's cache if it exists
            if let Some(ref mut rich_state) = app.rich_state {
                rich_state.image_cache.set_failed(&url, error);
            }
        }
    }
}

/// Handle completed refresh result
async fn handle_refresh_result(
    app: &mut App,
    result: RefreshResult,
    _client: &DaemonClient,
    data_dir: Option<&PathBuf>,
) -> Result<()> {
    app.is_refreshing = false;

    match result {
        RefreshResult::Success { new_count } => {
            // Reload data from daemon
            load_feeds(app).await?;
            init_rich_article_state(app, data_dir);
            if new_count > 0 {
                app.set_status(format!("Refreshed: {} new articles", new_count));
            } else {
                app.set_status("Refreshed: no new articles");
            }
        }
        RefreshResult::Failure { error } => {
            app.set_status(format!("Refresh failed: {}", error));
        }
    }

    Ok(())
}

/// Spawn an async task to download an image
fn spawn_image_load(
    url: String,
    tx: mpsc::UnboundedSender<ImageLoadResult>,
    data_dir: Option<PathBuf>,
    _cache: &kenseader_tui::rich_content::ArticleImageCache,
) {
    // First, try disk cache synchronously
    if let Some(ref dir) = data_dir {
        let disk_cache = kenseader_tui::rich_content::ImageDiskCache::new(dir);
        if let Ok(disk_cache) = disk_cache {
            if let Some(img) = disk_cache.load(&url) {
                let cache_path = Some(disk_cache.cache_path(&url));
                let _ = tx.send(ImageLoadResult::Success {
                    url,
                    image: img,
                    bytes: Vec::new(), // No need to re-save
                    cache_path,
                });
                return;
            }
        }
    }

    // Spawn async download
    let data_dir_clone = data_dir.clone();
    tokio::spawn(async move {
        match download_image(&url).await {
            Ok((bytes, image)) => {
                // Calculate cache path for external viewer
                let cache_path = data_dir_clone.and_then(|dir| {
                    kenseader_tui::rich_content::ImageDiskCache::new(&dir)
                        .ok()
                        .map(|dc| dc.cache_path(&url))
                });
                let _ = tx.send(ImageLoadResult::Success {
                    url,
                    image,
                    bytes,
                    cache_path,
                });
            }
            Err(e) => {
                let _ = tx.send(ImageLoadResult::Failure { url, error: e });
            }
        }
    });
}

/// Preload range: preload images for ±2 articles around current selection
const PRELOAD_RANGE: usize = 2;
/// Maximum concurrent preload requests per frame
const MAX_PRELOAD_CONCURRENT: usize = 3;

/// Process preloading of images for nearby articles
fn process_preload(
    app: &mut App,
    tx: &mpsc::UnboundedSender<ImageLoadResult>,
    data_dir: Option<&PathBuf>,
) {
    let range = app.get_preload_article_range(PRELOAD_RANGE);
    let mut count = 0;

    // Iterate through articles in the preload range
    for idx in range {
        if count >= MAX_PRELOAD_CONCURRENT {
            break;
        }

        if let Some(article) = app.articles.get(idx).cloned() {
            let urls = App::get_article_image_urls(&article);

            for url in urls {
                if count >= MAX_PRELOAD_CONCURRENT {
                    break;
                }

                // Skip if already ready or loading
                if app.preload_cache.is_ready(&url) || app.preload_cache.is_loading(&url) {
                    continue;
                }

                // Also skip if current article's cache already has it
                if let Some(ref rich_state) = app.rich_state {
                    if rich_state.image_cache.is_ready(&url) || rich_state.image_cache.is_loading(&url) {
                        continue;
                    }
                }

                // Mark as loading and spawn download
                app.preload_cache.start_loading(&url);
                spawn_preload_image(url, tx.clone(), data_dir.cloned());
                count += 1;
            }
        }
    }
}

/// Spawn an async task to download an image for preloading
fn spawn_preload_image(
    url: String,
    tx: mpsc::UnboundedSender<ImageLoadResult>,
    data_dir: Option<PathBuf>,
) {
    // First, try disk cache synchronously
    if let Some(ref dir) = data_dir {
        if let Ok(disk_cache) = kenseader_tui::rich_content::ImageDiskCache::new(dir) {
            if let Some(img) = disk_cache.load(&url) {
                let cache_path = Some(disk_cache.cache_path(&url));
                let _ = tx.send(ImageLoadResult::Success {
                    url,
                    image: img,
                    bytes: Vec::new(),
                    cache_path,
                });
                return;
            }
        }
    }

    // Spawn async download
    let data_dir_clone = data_dir.clone();
    tokio::spawn(async move {
        match download_image(&url).await {
            Ok((bytes, image)) => {
                let cache_path = data_dir_clone.and_then(|dir| {
                    kenseader_tui::rich_content::ImageDiskCache::new(&dir)
                        .ok()
                        .map(|dc| dc.cache_path(&url))
                });
                let _ = tx.send(ImageLoadResult::Success {
                    url,
                    image,
                    bytes,
                    cache_path,
                });
            }
            Err(e) => {
                let _ = tx.send(ImageLoadResult::Failure { url, error: e });
            }
        }
    });
}

async fn load_feeds(app: &mut App) -> Result<()> {
    app.feeds = app.client.list_feeds().await?;

    if !app.feeds.is_empty() {
        // Ensure selected feed is valid for current view mode
        ensure_valid_feed_selection(app);
        load_articles(app).await?;
    }

    Ok(())
}

/// Ensure selected feed is valid for current view mode
fn ensure_valid_feed_selection(app: &mut App) {
    let visible_feeds = app.visible_feeds();
    if visible_feeds.is_empty() {
        // No visible feeds, but keep selected_feed pointing to a valid feed if any exist
        if !app.feeds.is_empty() && app.selected_feed >= app.feeds.len() {
            app.selected_feed = 0;
        }
        return;
    }

    // Check if current selection is still visible
    if app.actual_to_visible_feed_index(app.selected_feed).is_none() {
        // Current feed is hidden, select first visible feed
        if let Some(actual_idx) = app.visible_to_actual_feed_index(0) {
            app.selected_feed = actual_idx;
        }
    }
}

async fn load_articles(app: &mut App) -> Result<()> {
    load_articles_preserve_selection(app, false).await
}

async fn load_articles_preserve_selection(app: &mut App, preserve: bool) -> Result<()> {
    if let Some(feed) = app.current_feed() {
        let feed_idx = app.selected_feed;
        let unread_only = matches!(app.view_mode, ViewMode::UnreadOnly);
        let prev_selected = app.selected_article;

        app.articles = app.client.list_articles(Some(feed.id), unread_only).await?;

        // Sync unread_count with actual article data
        let actual_unread_count = if unread_only {
            // In unread-only mode, all articles in the list are unread
            app.articles.len() as u32
        } else {
            // In all mode, count articles where is_read = false
            app.articles.iter().filter(|a| !a.is_read).count() as u32
        };

        // Update the feed's unread_count to match reality
        if let Some(feed) = app.feeds.get_mut(feed_idx) {
            feed.unread_count = actual_unread_count;
        }

        if preserve && prev_selected < app.articles.len() {
            app.selected_article = prev_selected;
        } else {
            app.selected_article = 0;
            app.detail_scroll = 0;
        }

        // Always clear rich state when articles change - it will be re-initialized on demand
        app.clear_rich_state();
    }

    Ok(())
}

/// Load articles for history navigation - ignores unread-only filter to find the target article
async fn load_articles_for_history(app: &mut App, target_article_id: Uuid) -> Result<bool> {
    let feed_id = match app.current_feed() {
        Some(feed) => feed.id,
        None => return Ok(false),
    };

    // First try to find in current filtered list
    let unread_only = matches!(app.view_mode, ViewMode::UnreadOnly);
    app.articles = app.client.list_articles(Some(feed_id), unread_only).await?;

    if let Some(idx) = app.find_article_index(target_article_id) {
        app.selected_article = idx;
        app.detail_scroll = 0;
        app.clear_rich_state();
        return Ok(true);
    }

    // If in unread-only mode and article not found, load all articles
    if unread_only {
        app.articles = app.client.list_articles(Some(feed_id), false).await?;

        if let Some(idx) = app.find_article_index(target_article_id) {
            app.selected_article = idx;
            app.detail_scroll = 0;
            app.clear_rich_state();
            // Temporarily switch to All mode to show the article
            app.view_mode = ViewMode::All;
            return Ok(true);
        }
    }

    Ok(false)
}

/// Initialize rich content state for the current article
fn init_rich_article_state(app: &mut App, data_dir: Option<&PathBuf>) {
    // Only initialize if image preview is enabled
    if !app.config.ui.image_preview {
        app.clear_rich_state();
        return;
    }

    if let Some(article) = app.current_article() {
        // Check if we already have state for this article
        // (Simple check: if rich_state exists and has content, keep it)
        if app.rich_state.is_some() {
            return;
        }

        // Parse HTML content or fall back to text
        let mut rich_state = if let Some(ref html) = article.content {
            RichArticleState::from_html(html, data_dir)
        } else if let Some(ref text) = article.content_text {
            RichArticleState::from_text(text, data_dir)
        } else {
            return; // No content to display
        };

        // Pre-fill images from preload cache (makes images appear instantly)
        for url in &rich_state.content.image_urls {
            if let Some(cached) = app.preload_cache.get(url) {
                rich_state.image_cache.set_loaded(
                    url,
                    cached.image.clone(),
                    cached.cache_path.clone(),
                );
            }
        }

        app.rich_state = Some(rich_state);
    } else {
        app.clear_rich_state();
    }
}

async fn handle_action(
    app: &mut App,
    action: Action,
    client: &DaemonClient,
    data_dir: Option<&PathBuf>,
    refresh_tx: mpsc::UnboundedSender<RefreshResult>,
) -> Result<()> {
    // Clear pending key on any action except PendingG
    if action != Action::PendingG && action != Action::JumpToTop {
        app.clear_pending_key();
    }

    match action {
        Action::Quit => {
            app.should_quit = true;
        }
        Action::FocusLeft => {
            let prev_focus = app.focus;
            app.focus_left();
            // Reload articles when returning from ArticleDetail to ArticleList in unread-only mode
            // This removes the just-read article from the filtered list
            if prev_focus == Focus::ArticleDetail
                && app.focus == Focus::ArticleList
                && matches!(app.view_mode, ViewMode::UnreadOnly)
            {
                // Ensure feed selection is valid (feed may be hidden if all articles read)
                ensure_valid_feed_selection(app);
                load_articles_preserve_selection(app, true).await?;
                // Re-initialize rich state since the article at the current index may have changed
                init_rich_article_state(app, data_dir);
            }
        }
        Action::FocusRight => {
            let prev_focus = app.focus;
            app.focus_right();
            // Auto mark-read when entering article detail
            if prev_focus == Focus::ArticleList && app.focus == Focus::ArticleDetail {
                // Record history before entering article
                app.push_history();
                // Reset scroll to top (like vim 'gg')
                app.detail_scroll = 0;
                if let Some(article) = app.current_article() {
                    if !article.is_read {
                        let article_id = article.id;
                        client.mark_read(article_id).await?;
                        // Update local state without reloading (keeps article visible in unread-only mode)
                        if let Some(article) = app.current_article_mut() {
                            article.is_read = true;
                        }
                        // Decrement feed unread count
                        if let Some(feed) = app.current_feed_mut() {
                            feed.unread_count = feed.unread_count.saturating_sub(1);
                        }
                    }
                }
                // Initialize rich content state
                init_rich_article_state(app, data_dir);
            }
        }
        Action::MoveUp => {
            let prev_feed = app.selected_feed;
            let prev_article = app.selected_article;

            // For Subscriptions, use visible feed navigation
            if app.focus == Focus::Subscriptions {
                let visible_feeds = app.visible_feeds();
                if !visible_feeds.is_empty() {
                    let current_visible = app
                        .actual_to_visible_feed_index(app.selected_feed)
                        .unwrap_or(0);
                    let next_visible = current_visible.saturating_sub(1);
                    if let Some(actual_idx) = app.visible_to_actual_feed_index(next_visible) {
                        app.selected_feed = actual_idx;
                    }
                }
                // Update visual selection if in visual mode
                app.update_visual_selection_feeds();
            } else {
                // Record history before switching articles in ArticleDetail
                if app.focus == Focus::ArticleDetail && app.selected_article > 0 {
                    app.push_history();
                }
                app.move_up();
                // Update visual selection if in visual mode (for ArticleList)
                if matches!(app.focus, Focus::ArticleList | Focus::ArticleDetail) {
                    app.update_visual_selection_articles();
                }
            }

            if app.focus == Focus::Subscriptions && prev_feed != app.selected_feed {
                // Clear preload cache when switching feeds
                app.preload_cache.clear();
                load_articles(app).await?;
                // Initialize rich state for the first article in the new feed
                init_rich_article_state(app, data_dir);
            }
            // Re-initialize rich state when article changes to ensure consistent rendering
            if app.focus == Focus::ArticleList && prev_article != app.selected_article {
                app.detail_scroll = 0; // Reset scroll to top
                app.clear_rich_state();
                init_rich_article_state(app, data_dir);
            }
            // Handle article change in ArticleDetail (with auto mark-read)
            if app.focus == Focus::ArticleDetail && prev_article != app.selected_article {
                app.detail_scroll = 0;
                app.clear_rich_state();
                // Auto mark-read
                if let Some(article) = app.current_article() {
                    if !article.is_read {
                        let article_id = article.id;
                        client.mark_read(article_id).await?;
                        if let Some(article) = app.current_article_mut() {
                            article.is_read = true;
                        }
                        if let Some(feed) = app.current_feed_mut() {
                            feed.unread_count = feed.unread_count.saturating_sub(1);
                        }
                    }
                }
                init_rich_article_state(app, data_dir);
            }
        }
        Action::MoveDown => {
            let prev_feed = app.selected_feed;
            let prev_article = app.selected_article;

            // For Subscriptions, use visible feed navigation
            if app.focus == Focus::Subscriptions {
                let visible_feeds = app.visible_feeds();
                if !visible_feeds.is_empty() {
                    let current_visible = app
                        .actual_to_visible_feed_index(app.selected_feed)
                        .unwrap_or(0);
                    let next_visible = (current_visible + 1).min(visible_feeds.len() - 1);
                    if let Some(actual_idx) = app.visible_to_actual_feed_index(next_visible) {
                        app.selected_feed = actual_idx;
                    }
                }
                // Update visual selection if in visual mode
                app.update_visual_selection_feeds();
            } else {
                // Record history before switching articles in ArticleDetail
                if app.focus == Focus::ArticleDetail && app.selected_article < app.articles.len().saturating_sub(1) {
                    app.push_history();
                }
                app.move_down();
                // Update visual selection if in visual mode (for ArticleList)
                if matches!(app.focus, Focus::ArticleList | Focus::ArticleDetail) {
                    app.update_visual_selection_articles();
                }
            }

            if app.focus == Focus::Subscriptions && prev_feed != app.selected_feed {
                // Clear preload cache when switching feeds
                app.preload_cache.clear();
                load_articles(app).await?;
                // Initialize rich state for the first article in the new feed
                init_rich_article_state(app, data_dir);
            }
            // Re-initialize rich state when article changes to ensure consistent rendering
            if app.focus == Focus::ArticleList && prev_article != app.selected_article {
                app.detail_scroll = 0; // Reset scroll to top
                app.clear_rich_state();
                init_rich_article_state(app, data_dir);
            }
            // Handle article change in ArticleDetail (with auto mark-read)
            if app.focus == Focus::ArticleDetail && prev_article != app.selected_article {
                app.detail_scroll = 0;
                app.clear_rich_state();
                // Auto mark-read
                if let Some(article) = app.current_article() {
                    if !article.is_read {
                        let article_id = article.id;
                        client.mark_read(article_id).await?;
                        if let Some(article) = app.current_article_mut() {
                            article.is_read = true;
                        }
                        if let Some(feed) = app.current_feed_mut() {
                            feed.unread_count = feed.unread_count.saturating_sub(1);
                        }
                    }
                }
                init_rich_article_state(app, data_dir);
            }
        }
        Action::ScrollHalfPageDown => {
            app.scroll_half_page_down();
            // Update visual selection if in visual mode
            match app.focus {
                Focus::ArticleList | Focus::ArticleDetail => {
                    app.update_visual_selection_articles();
                }
                Focus::Subscriptions => {
                    app.update_visual_selection_feeds();
                }
            }
        }
        Action::ScrollHalfPageUp => {
            app.scroll_half_page_up();
            // Update visual selection if in visual mode
            match app.focus {
                Focus::ArticleList | Focus::ArticleDetail => {
                    app.update_visual_selection_articles();
                }
                Focus::Subscriptions => {
                    app.update_visual_selection_feeds();
                }
            }
        }
        Action::ScrollPageDown => {
            // Full page scroll using viewport height
            app.scroll_full_page_down();
            // Update visual selection if in visual mode
            match app.focus {
                Focus::ArticleList | Focus::ArticleDetail => {
                    app.update_visual_selection_articles();
                }
                Focus::Subscriptions => {
                    app.update_visual_selection_feeds();
                }
            }
        }
        Action::ScrollPageUp => {
            // Full page scroll using viewport height
            app.scroll_full_page_up();
            // Update visual selection if in visual mode
            match app.focus {
                Focus::ArticleList | Focus::ArticleDetail => {
                    app.update_visual_selection_articles();
                }
                Focus::Subscriptions => {
                    app.update_visual_selection_feeds();
                }
            }
        }
        Action::JumpToTop => {
            let prev_article = app.selected_article;
            // Record history before jumping in ArticleDetail
            if app.focus == Focus::ArticleDetail && app.selected_article > 0 {
                app.push_history();
            }
            app.jump_to_top();
            app.clear_pending_key();
            // Update visual selection if in visual mode
            match app.focus {
                Focus::ArticleList | Focus::ArticleDetail => {
                    app.update_visual_selection_articles();
                }
                Focus::Subscriptions => {
                    app.update_visual_selection_feeds();
                }
            }
            // Handle article change in ArticleDetail (with auto mark-read)
            if app.focus == Focus::ArticleDetail && prev_article != app.selected_article {
                app.detail_scroll = 0;
                app.clear_rich_state();
                if let Some(article) = app.current_article() {
                    if !article.is_read {
                        let article_id = article.id;
                        client.mark_read(article_id).await?;
                        if let Some(article) = app.current_article_mut() {
                            article.is_read = true;
                        }
                        if let Some(feed) = app.current_feed_mut() {
                            feed.unread_count = feed.unread_count.saturating_sub(1);
                        }
                    }
                }
                init_rich_article_state(app, data_dir);
            }
        }
        Action::JumpToBottom => {
            let prev_article = app.selected_article;
            // Record history before jumping in ArticleDetail
            if app.focus == Focus::ArticleDetail && app.selected_article < app.articles.len().saturating_sub(1) {
                app.push_history();
            }
            app.jump_to_bottom();
            // Update visual selection if in visual mode
            match app.focus {
                Focus::ArticleList | Focus::ArticleDetail => {
                    app.update_visual_selection_articles();
                }
                Focus::Subscriptions => {
                    app.update_visual_selection_feeds();
                }
            }
            // Handle article change in ArticleDetail (with auto mark-read)
            if app.focus == Focus::ArticleDetail && prev_article != app.selected_article {
                app.detail_scroll = 0;
                app.clear_rich_state();
                if let Some(article) = app.current_article() {
                    if !article.is_read {
                        let article_id = article.id;
                        client.mark_read(article_id).await?;
                        if let Some(article) = app.current_article_mut() {
                            article.is_read = true;
                        }
                        if let Some(feed) = app.current_feed_mut() {
                            feed.unread_count = feed.unread_count.saturating_sub(1);
                        }
                    }
                }
                init_rich_article_state(app, data_dir);
            }
        }
        Action::PendingG => {
            app.pending_key = Some('g');
        }
        Action::Select => {
            if app.focus == Focus::ArticleList {
                // Record history before entering article
                app.push_history();
                // Mark as read and switch to detail
                if let Some(article) = app.current_article() {
                    if !article.is_read {
                        let article_id = article.id;
                        client.mark_read(article_id).await?;
                        // Update local state without reloading (keeps article visible in unread-only mode)
                        if let Some(article) = app.current_article_mut() {
                            article.is_read = true;
                        }
                        // Decrement feed unread count
                        if let Some(feed) = app.current_feed_mut() {
                            feed.unread_count = feed.unread_count.saturating_sub(1);
                        }
                    }
                }
                app.focus = Focus::ArticleDetail;
                // Initialize rich content state
                init_rich_article_state(app, data_dir);
            }
        }
        Action::OpenInBrowser => {
            if let Some(article) = app.current_article() {
                if let Some(url) = &article.url {
                    if let Err(e) = open::that(url) {
                        app.set_status(format!("Failed to open browser: {}", e));
                    }
                }
            }
        }
        Action::ToggleSaved => {
            if let Some(article) = app.current_article() {
                let saved = client.toggle_saved(article.id).await?;
                app.set_status(if saved { "Article saved" } else { "Article unsaved" });
                load_articles_preserve_selection(app, true).await?;
                init_rich_article_state(app, data_dir);
            }
        }
        Action::Delete => {
            // Batch delete takes priority if feeds are selected (regardless of current focus)
            if !app.selected_feeds.is_empty() {
                app.mode = Mode::BatchDeleteConfirm;
            } else if app.focus == Focus::Subscriptions {
                if let Some(feed) = app.current_feed() {
                    // Single delete
                    app.mode = Mode::DeleteConfirm(feed.id);
                }
            }
        }
        Action::Confirm => {
            match &app.mode {
                Mode::DeleteConfirm(feed_id) => {
                    let feed_id = *feed_id;
                    client.delete_feed(feed_id).await?;
                    app.mode = Mode::Normal;
                    load_feeds(app).await?;
                    init_rich_article_state(app, data_dir);
                    app.set_status("Feed deleted");
                }
                Mode::BatchDeleteConfirm => {
                    // Batch delete selected feeds
                    let indices: Vec<usize> = app.selected_feeds.iter().cloned().collect();
                    let mut deleted_count = 0;
                    let mut errors = Vec::new();

                    for &idx in &indices {
                        if let Some(feed) = app.feeds.get(idx) {
                            let feed_id = feed.id;
                            match client.delete_feed(feed_id).await {
                                Ok(_) => {
                                    deleted_count += 1;
                                }
                                Err(e) => {
                                    errors.push(format!("{}", e));
                                }
                            }
                        }
                    }

                    app.mode = Mode::Normal;
                    app.clear_feed_selection();
                    load_feeds(app).await?;
                    init_rich_article_state(app, data_dir);

                    if errors.is_empty() {
                        app.set_status(format!("Deleted {} feeds", deleted_count));
                    } else {
                        app.set_status(format!("Deleted {} feeds, {} errors", deleted_count, errors.len()));
                    }
                }
                Mode::SearchForward(_) | Mode::SearchBackward(_) => {
                    app.execute_search();
                    let match_count = app.search_matches.len();
                    app.mode = Mode::Normal;
                    if match_count > 0 {
                        app.set_status(format!("{} matches found", match_count));
                    } else {
                        app.set_status("No matches found");
                    }
                }
                _ => {}
            }
        }
        Action::Cancel => {
            // Clear selected feeds when canceling batch delete
            if matches!(app.mode, Mode::BatchDeleteConfirm) {
                app.clear_feed_selection();
                app.set_status("Batch delete canceled");
            }
            app.mode = Mode::Normal;
            app.search_query.clear();
        }
        Action::ToggleUnreadOnly => {
            app.toggle_view_mode();
            // Ensure selected feed is valid for new view mode
            ensure_valid_feed_selection(app);
            load_articles(app).await?;
            init_rich_article_state(app, data_dir);
        }
        Action::ExitMode => {
            app.mode = Mode::Normal;
            app.view_mode = ViewMode::All;
            load_articles(app).await?;
            init_rich_article_state(app, data_dir);
        }
        Action::StartSearchForward => {
            app.mode = Mode::SearchForward(String::new());
            app.search_query.clear();
        }
        Action::StartSearchBackward => {
            app.mode = Mode::SearchBackward(String::new());
            app.search_query.clear();
        }
        Action::InputChar(c) => {
            app.search_query.push(c);
            // Real-time search: execute search as user types
            app.execute_search();
        }
        Action::Backspace => {
            app.search_query.pop();
            // Real-time search: update matches after backspace
            app.execute_search();
        }
        Action::Refresh => {
            // Don't start another refresh if one is already in progress
            if app.is_refreshing {
                app.set_status("Refresh already in progress...");
            } else {
                app.is_refreshing = true;
                app.set_status("Refreshing feeds...");

                // Clone what we need for the spawned task
                let client = app.client.clone();
                let tx = refresh_tx.clone();

                // Spawn refresh as background task
                tokio::spawn(async move {
                    match client.refresh(None).await {
                        Ok(new_count) => {
                            let _ = tx.send(RefreshResult::Success { new_count });
                        }
                        Err(e) => {
                            let _ = tx.send(RefreshResult::Failure {
                                error: e.to_string(),
                            });
                        }
                    }
                });
            }
        }
        Action::NextMatch => {
            app.next_search_match();
            if !app.search_matches.is_empty() {
                app.set_status(format!(
                    "Match {}/{}",
                    app.current_match + 1,
                    app.search_matches.len()
                ));
            }
        }
        Action::PrevMatch => {
            app.prev_search_match();
            if !app.search_matches.is_empty() {
                app.set_status(format!(
                    "Match {}/{}",
                    app.current_match + 1,
                    app.search_matches.len()
                ));
            }
        }
        Action::ToggleRead => {
            // Check for batch operation
            if !app.selected_articles.is_empty() {
                // Batch toggle read status
                let indices: Vec<usize> = app.selected_articles.iter().cloned().collect();
                let mut toggled_count = 0;
                let mut errors = Vec::new();

                for &idx in &indices {
                    if let Some(article) = app.articles.get(idx) {
                        let article_id = article.id;
                        let was_read = article.is_read;

                        let result = if was_read {
                            client.mark_unread(article_id).await
                        } else {
                            client.mark_read(article_id).await
                        };

                        match result {
                            Ok(_) => {
                                // Update local state
                                if let Some(article) = app.articles.get_mut(idx) {
                                    article.is_read = !was_read;
                                }
                                // Update feed unread count
                                if let Some(feed) = app.current_feed_mut() {
                                    if was_read {
                                        feed.unread_count += 1;
                                    } else {
                                        feed.unread_count = feed.unread_count.saturating_sub(1);
                                    }
                                }
                                toggled_count += 1;
                            }
                            Err(e) => {
                                errors.push(format!("{}", e));
                            }
                        }
                    }
                }

                app.clear_article_selection();
                if errors.is_empty() {
                    app.set_status(format!("Toggled {} articles", toggled_count));
                } else {
                    app.set_status(format!("Toggled {} articles, {} errors", toggled_count, errors.len()));
                }
            } else {
                // Single article toggle (original behavior)
                if let Some(article) = app.current_article() {
                    let article_id = article.id;
                    let was_read = article.is_read;

                    let result = if was_read {
                        client.mark_unread(article_id).await
                    } else {
                        client.mark_read(article_id).await
                    };

                    match result {
                        Ok(_) => {
                            // Update local article state
                            if let Some(article) = app.current_article_mut() {
                                article.is_read = !was_read;
                            }
                            // Update feed unread count
                            if let Some(feed) = app.current_feed_mut() {
                                if was_read {
                                    feed.unread_count += 1;
                                } else {
                                    feed.unread_count = feed.unread_count.saturating_sub(1);
                                }
                            }
                            let status = if was_read {
                                "Marked as unread"
                            } else {
                                "Marked as read"
                            };
                            app.set_status(status);
                        }
                        Err(e) => {
                            app.set_status(format!("Failed to toggle read status: {}", e));
                        }
                    }
                }
            }
        }
        Action::HistoryBack => {
            // Debug: show history state
            let history_len = app.read_history.len();
            let history_pos = app.history_position;

            if let Some((feed_id, article_id)) = app.history_back() {
                // Find feed by ID
                if let Some(feed_idx) = app.find_feed_index(feed_id) {
                    if feed_idx != app.selected_feed {
                        app.selected_feed = feed_idx;
                    }
                    // Load articles and find the target article (ignores unread-only if needed)
                    if load_articles_for_history(app, article_id).await? {
                        init_rich_article_state(app, data_dir);
                        app.set_status("← Back");
                    } else {
                        app.set_status(format!("Article not found (history: {}/{})", history_pos, history_len));
                    }
                } else {
                    app.set_status(format!("Feed not found (history: {}/{})", history_pos, history_len));
                }
            } else {
                app.set_status(format!("No history to go back (pos: {}, len: {})", history_pos, history_len));
            }
        }
        Action::HistoryForward => {
            if let Some((feed_id, article_id)) = app.history_forward() {
                // Find feed by ID
                if let Some(feed_idx) = app.find_feed_index(feed_id) {
                    if feed_idx != app.selected_feed {
                        app.selected_feed = feed_idx;
                    }
                    // Load articles and find the target article (ignores unread-only if needed)
                    if load_articles_for_history(app, article_id).await? {
                        init_rich_article_state(app, data_dir);
                        app.set_status("→ Forward");
                    } else {
                        app.set_status("Article not found in history");
                    }
                } else {
                    app.set_status("Feed not found in history");
                }
            }
        }
        Action::ToggleSelect => {
            match app.focus {
                Focus::ArticleList | Focus::ArticleDetail => {
                    app.toggle_article_selection(app.selected_article);
                    // Move to next item (if not at last item)
                    if app.selected_article < app.articles.len().saturating_sub(1) {
                        app.selected_article += 1;
                        app.detail_scroll = 0;
                        app.clear_rich_state();
                        init_rich_article_state(app, data_dir);
                    }
                }
                Focus::Subscriptions => {
                    // Get info before borrowing mutably
                    let visible_len = app.visible_feeds().len();
                    let visible_idx = app.actual_to_visible_feed_index(app.selected_feed);
                    let current_feed = app.selected_feed;

                    if let Some(visible_idx) = visible_idx {
                        // Toggle selection using actual index
                        app.toggle_feed_selection(current_feed);
                        // Move to next item (if not at last item)
                        if visible_idx < visible_len.saturating_sub(1) {
                            let next_visible = visible_idx + 1;
                            if let Some(actual_idx) = app.visible_to_actual_feed_index(next_visible) {
                                app.selected_feed = actual_idx;
                                load_articles(app).await?;
                                init_rich_article_state(app, data_dir);
                            }
                        }
                    }
                }
            }
        }
        Action::VisualMode => {
            match app.focus {
                Focus::ArticleList | Focus::ArticleDetail => {
                    if app.visual_start_article.is_some() {
                        // Exit visual mode
                        app.visual_start_article = None;
                        app.set_status("Visual mode ended");
                    } else {
                        // Enter visual mode, record start position
                        app.visual_start_article = Some(app.selected_article);
                        app.selected_articles.clear();
                        app.selected_articles.insert(app.selected_article);
                        app.set_status("-- VISUAL --");
                    }
                }
                Focus::Subscriptions => {
                    if app.visual_start_feed.is_some() {
                        // Exit visual mode
                        app.visual_start_feed = None;
                        app.set_status("Visual mode ended");
                    } else {
                        // Enter visual mode, record start position
                        app.visual_start_feed = Some(app.selected_feed);
                        app.selected_feeds.clear();
                        app.selected_feeds.insert(app.selected_feed);
                        app.set_status("-- VISUAL --");
                    }
                }
            }
        }
        Action::ClearSelection => {
            app.clear_article_selection();
            app.clear_feed_selection();
            app.set_status("Selection cleared");
        }
        // Image navigation and viewing actions
        Action::ViewImage => {
            // Enter fullscreen image viewer mode
            if let Some(ref rich_state) = app.rich_state {
                let image_count = rich_state.content.image_urls.len();
                if image_count > 0 {
                    // Use focused image index or default to first image
                    let index = rich_state.focused_image_index().unwrap_or(0);
                    app.mode = Mode::ImageViewer(index);
                } else {
                    app.set_status("No images in this article");
                }
            }
        }
        Action::OpenImage => {
            // Smart open: image -> external viewer, link -> browser
            // First check if in ImageViewer mode
            if let Mode::ImageViewer(idx) = app.mode {
                // Open current image in external viewer
                if let Some(ref rich_state) = app.rich_state {
                    if let Some(url) = rich_state.content.image_urls.get(idx) {
                        if let Some(cached) = rich_state.image_cache.get(url) {
                            if let Some(ref path) = cached.cache_path {
                                if path.exists() {
                                    if let Err(e) = open::that(path) {
                                        app.set_status(format!("Failed to open image: {}", e));
                                    } else {
                                        app.set_status("Opening image in external viewer...");
                                    }
                                } else {
                                    app.set_status("Image not cached locally");
                                }
                            } else {
                                app.set_status("Image cache path not available");
                            }
                        } else {
                            app.set_status("Image not loaded yet");
                        }
                    }
                }
            } else if let Some(ref rich_state) = app.rich_state {
                // Normal mode: check focused item type
                match rich_state.get_focused_item() {
                    Some(FocusableItem::Image { url_index }) => {
                        // Open image in external viewer
                        if let Some(url) = rich_state.content.image_urls.get(*url_index) {
                            if let Some(cached) = rich_state.image_cache.get(url) {
                                if let Some(ref path) = cached.cache_path {
                                    if path.exists() {
                                        if let Err(e) = open::that(path) {
                                            app.set_status(format!("Failed to open image: {}", e));
                                        } else {
                                            app.set_status("Opening image in external viewer...");
                                        }
                                    } else {
                                        app.set_status("Image not cached locally");
                                    }
                                } else {
                                    app.set_status("Image cache path not available");
                                }
                            } else {
                                app.set_status("Image not loaded yet");
                            }
                        }
                    }
                    Some(FocusableItem::Link { url, text, .. }) => {
                        // Open link in browser
                        if let Err(e) = open::that(url) {
                            app.set_status(format!("Failed to open link: {}", e));
                        } else {
                            let display = if text.len() > 30 {
                                format!("{}...", &text[..27])
                            } else {
                                text.clone()
                            };
                            app.set_status(format!("Opening: {}", display));
                        }
                    }
                    None => {
                        app.set_status("No item focused. Press Tab to focus an item.");
                    }
                }
            }
        }
        Action::NextImage => {
            let status_msg: Option<String> = if let Some(ref mut rich_state) = app.rich_state {
                let focusable_count = rich_state.content.focusable_items.len();
                let image_count = rich_state.content.image_urls.len();

                match &mut app.mode {
                    Mode::ImageViewer(idx) => {
                        // In image viewer mode, only navigate between images
                        if image_count > 0 {
                            *idx = (*idx + 1) % image_count;
                        }
                        None
                    }
                    _ => {
                        // Navigate through all focusable items (images + links)
                        if focusable_count > 0 {
                            rich_state.focused_item = Some(
                                rich_state
                                    .focused_item
                                    .map(|i| (i + 1) % focusable_count)
                                    .unwrap_or(0),
                            );

                            // Show status based on item type
                            let idx = rich_state.focused_item.unwrap();
                            match &rich_state.content.focusable_items[idx] {
                                FocusableItem::Image { url_index } => {
                                    Some(format!("Image {}/{}", url_index + 1, image_count))
                                }
                                FocusableItem::Link { text, .. } => {
                                    let display_text = if text.len() > 40 {
                                        format!("{}...", &text[..37])
                                    } else {
                                        text.clone()
                                    };
                                    Some(format!("Link: {}", display_text))
                                }
                            }
                        } else {
                            Some("No focusable items in this article".to_string())
                        }
                    }
                }
            } else {
                None
            };
            if let Some(msg) = status_msg {
                app.set_status(msg);
            }
        }
        Action::PrevImage => {
            let status_msg: Option<String> = if let Some(ref mut rich_state) = app.rich_state {
                let focusable_count = rich_state.content.focusable_items.len();
                let image_count = rich_state.content.image_urls.len();

                match &mut app.mode {
                    Mode::ImageViewer(idx) => {
                        // In image viewer mode, only navigate between images
                        if image_count > 0 {
                            *idx = if *idx == 0 { image_count - 1 } else { *idx - 1 };
                        }
                        None
                    }
                    _ => {
                        // Navigate through all focusable items (images + links)
                        if focusable_count > 0 {
                            rich_state.focused_item = Some(
                                rich_state
                                    .focused_item
                                    .map(|i| if i == 0 { focusable_count - 1 } else { i - 1 })
                                    .unwrap_or(focusable_count - 1),
                            );

                            // Show status based on item type
                            let idx = rich_state.focused_item.unwrap();
                            match &rich_state.content.focusable_items[idx] {
                                FocusableItem::Image { url_index } => {
                                    Some(format!("Image {}/{}", url_index + 1, image_count))
                                }
                                FocusableItem::Link { text, .. } => {
                                    let display_text = if text.len() > 40 {
                                        format!("{}...", &text[..37])
                                    } else {
                                        text.clone()
                                    };
                                    Some(format!("Link: {}", display_text))
                                }
                            }
                        } else {
                            Some("No focusable items in this article".to_string())
                        }
                    }
                }
            } else {
                None
            };
            if let Some(msg) = status_msg {
                app.set_status(msg);
            }
        }
        Action::ExitImageViewer => {
            if matches!(app.mode, Mode::ImageViewer(_)) {
                // Clear fullscreen image before returning to article view
                app.image_renderer.clear_all();
                app.mode = Mode::Normal;
            }
        }
        Action::None => {}
    }

    Ok(())
}
