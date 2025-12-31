use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
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
    storage::{ArticleRepository, Database, FeedRepository},
    AppConfig,
};
use kenseader_tui::{
    app::{App, Focus, Mode, RichArticleState, ViewMode},
    event::{AppEvent, EventHandler, ImageLoadResult},
    input::{handle_key_event, Action},
    rich_content::download_image,
    widgets::{ArticleDetailWidget, ArticleListWidget, StatusBarWidget, SubscriptionsWidget},
};

pub async fn run(db: Arc<Database>, config: Arc<AppConfig>) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(db.clone(), config.clone());

    // Load initial data
    load_feeds(&mut app).await?;

    // Create event handler
    let event_handler = EventHandler::new(config.ui.tick_rate_ms);

    // Create channel for async image loading results
    let (img_tx, mut img_rx) = mpsc::unbounded_channel::<ImageLoadResult>();

    // Get data directory for disk cache
    let data_dir = dirs::data_dir()
        .map(|d| d.join("kenseader"))
        .or_else(|| dirs::home_dir().map(|d| d.join(".kenseader")));

    // Main loop
    loop {
        // Process any completed image loads (non-blocking)
        while let Ok(result) = img_rx.try_recv() {
            handle_image_result(&mut app, result);
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

            // Main layout: content + status bar
            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(size);

            // Three-column layout with 1:4:5 ratio
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 10),  // Subscriptions
                    Constraint::Ratio(4, 10),  // Article list
                    Constraint::Ratio(5, 10),  // Article detail
                ])
                .split(main_layout[0]);

            // Render widgets
            SubscriptionsWidget::render(frame, columns[0], &app);
            ArticleListWidget::render(frame, columns[1], &app);
            ArticleDetailWidget::render(frame, columns[2], &mut app);
            StatusBarWidget::render(frame, main_layout[1], &app);
        })?;

        // Handle events
        if let Some(event) = event_handler.next()? {
            match event {
                AppEvent::Key(key) => {
                    let action = handle_key_event(key, &app);
                    handle_action(&mut app, action, &db, data_dir.as_ref()).await?;
                }
                AppEvent::Resize(_, _) => {
                    // Recalculate heights on resize
                    if let Some(ref mut rich_state) = app.rich_state {
                        rich_state.element_heights.clear();
                    }
                }
                AppEvent::Tick => {
                    // Periodic updates handled above
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
    if let Some(ref mut rich_state) = app.rich_state {
        match result {
            ImageLoadResult::Success { url, image, bytes } => {
                // Save to disk cache
                rich_state.image_cache.save_to_disk(&url, &bytes);
                // Store in memory cache
                rich_state.image_cache.set_loaded(&url, image);
            }
            ImageLoadResult::Failure { url, error } => {
                rich_state.image_cache.set_failed(&url, error);
            }
        }
    }
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
                let _ = tx.send(ImageLoadResult::Success {
                    url,
                    image: img,
                    bytes: Vec::new(), // No need to re-save
                });
                return;
            }
        }
    }

    // Spawn async download
    tokio::spawn(async move {
        match download_image(&url).await {
            Ok((bytes, image)) => {
                let _ = tx.send(ImageLoadResult::Success { url, image, bytes });
            }
            Err(e) => {
                let _ = tx.send(ImageLoadResult::Failure { url, error: e });
            }
        }
    });
}

async fn load_feeds(app: &mut App) -> Result<()> {
    let feed_repo = FeedRepository::new(&app.db);
    app.feeds = feed_repo.list_all().await?;

    if !app.feeds.is_empty() {
        load_articles(app).await?;
    }

    Ok(())
}

async fn load_articles(app: &mut App) -> Result<()> {
    load_articles_preserve_selection(app, false).await
}

async fn load_articles_preserve_selection(app: &mut App, preserve: bool) -> Result<()> {
    if let Some(feed) = app.current_feed() {
        let article_repo = ArticleRepository::new(&app.db);
        let unread_only = matches!(app.view_mode, ViewMode::UnreadOnly);
        let prev_selected = app.selected_article;

        app.articles = article_repo.list_by_feed(feed.id, unread_only).await?;

        if preserve && prev_selected < app.articles.len() {
            app.selected_article = prev_selected;
        } else {
            app.selected_article = 0;
            app.detail_scroll = 0;
        }

        if !preserve {
            app.rich_state = None; // Clear rich state when articles change
        }
    }

    Ok(())
}

/// Initialize rich content state for the current article
fn init_rich_article_state(app: &mut App, data_dir: Option<&PathBuf>) {
    // Only initialize if image preview is enabled
    if !app.config.ui.image_preview {
        app.rich_state = None;
        return;
    }

    if let Some(article) = app.current_article() {
        // Check if we already have state for this article
        // (Simple check: if rich_state exists and has content, keep it)
        if app.rich_state.is_some() {
            return;
        }

        // Parse HTML content or fall back to text
        let rich_state = if let Some(ref html) = article.content {
            RichArticleState::from_html(html, data_dir)
        } else if let Some(ref text) = article.content_text {
            RichArticleState::from_text(text, data_dir)
        } else {
            return; // No content to display
        };

        app.rich_state = Some(rich_state);
    } else {
        app.rich_state = None;
    }
}

async fn handle_action(
    app: &mut App,
    action: Action,
    db: &Database,
    data_dir: Option<&PathBuf>,
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
            app.focus_left();
        }
        Action::FocusRight => {
            let prev_focus = app.focus;
            app.focus_right();
            // Auto mark-read when entering article detail
            if prev_focus == Focus::ArticleList && app.focus == Focus::ArticleDetail {
                if let Some(article) = app.current_article() {
                    if !article.is_read {
                        let article_repo = ArticleRepository::new(db);
                        article_repo.mark_read(article.id).await?;
                        load_articles_preserve_selection(app, true).await?;
                    }
                }
                // Initialize rich content state
                init_rich_article_state(app, data_dir);
            }
        }
        Action::MoveUp => {
            let prev_feed = app.selected_feed;
            let prev_article = app.selected_article;
            app.move_up();
            if app.focus == Focus::Subscriptions && prev_feed != app.selected_feed {
                load_articles(app).await?;
            }
            // Clear rich state when article changes
            if app.focus == Focus::ArticleList && prev_article != app.selected_article {
                app.rich_state = None;
            }
        }
        Action::MoveDown => {
            let prev_feed = app.selected_feed;
            let prev_article = app.selected_article;
            app.move_down();
            if app.focus == Focus::Subscriptions && prev_feed != app.selected_feed {
                load_articles(app).await?;
            }
            // Clear rich state when article changes
            if app.focus == Focus::ArticleList && prev_article != app.selected_article {
                app.rich_state = None;
            }
        }
        Action::ScrollHalfPageDown => {
            app.scroll_half_page_down();
        }
        Action::ScrollHalfPageUp => {
            app.scroll_half_page_up();
        }
        Action::ScrollPageDown => {
            // Full page = 2 half pages
            app.scroll_half_page_down();
            app.scroll_half_page_down();
        }
        Action::ScrollPageUp => {
            app.scroll_half_page_up();
            app.scroll_half_page_up();
        }
        Action::JumpToTop => {
            app.jump_to_top();
            app.clear_pending_key();
        }
        Action::JumpToBottom => {
            app.jump_to_bottom();
        }
        Action::PendingG => {
            app.pending_key = Some('g');
        }
        Action::Select => {
            if app.focus == Focus::ArticleList {
                // Mark as read and switch to detail
                if let Some(article) = app.current_article() {
                    let article_repo = ArticleRepository::new(db);
                    article_repo.mark_read(article.id).await?;
                    // Reload to reflect change, preserving selection
                    load_articles_preserve_selection(app, true).await?;
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
                let article_repo = ArticleRepository::new(db);
                let saved = article_repo.toggle_saved(article.id).await?;
                app.set_status(if saved { "Article saved" } else { "Article unsaved" });
                load_articles_preserve_selection(app, true).await?;
            }
        }
        Action::Delete => {
            if app.focus == Focus::Subscriptions {
                if let Some(feed) = app.current_feed() {
                    app.mode = Mode::DeleteConfirm(feed.id);
                }
            }
        }
        Action::Confirm => {
            match &app.mode {
                Mode::DeleteConfirm(feed_id) => {
                    let feed_id = *feed_id;
                    let feed_repo = FeedRepository::new(db);
                    feed_repo.delete(feed_id).await?;
                    app.mode = Mode::Normal;
                    load_feeds(app).await?;
                    app.set_status("Feed deleted");
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
            app.mode = Mode::Normal;
            app.search_query.clear();
        }
        Action::ToggleUnreadOnly => {
            app.toggle_view_mode();
            load_articles(app).await?;
        }
        Action::ExitMode => {
            app.mode = Mode::Normal;
            app.view_mode = ViewMode::All;
            load_articles(app).await?;
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
            app.set_status("Refreshing...");
            // This would ideally be done in a background task
            load_feeds(app).await?;
            app.set_status("Refreshed");
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
        Action::None => {}
    }

    Ok(())
}
