use std::io;
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

use kenseader_core::{
    storage::{ArticleRepository, Database, FeedRepository},
    AppConfig,
};
use kenseader_tui::{
    app::{App, Focus, Mode, ViewMode},
    event::{AppEvent, EventHandler},
    input::{handle_key_event, Action},
    widgets::{ArticleDetailWidget, ArticleListWidget, StatusBarWidget, SubscriptionsWidget},
    ImageCache, load_image,
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

    // Main loop
    loop {
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
            ArticleDetailWidget::render(frame, columns[2], &app);
            StatusBarWidget::render(frame, main_layout[1], &app);
        })?;

        // Handle events
        if let Some(event) = event_handler.next()? {
            match event {
                AppEvent::Key(key) => {
                    let action = handle_key_event(key, &app);
                    handle_action(&mut app, action, &db).await?;
                }
                AppEvent::Resize(_, _) => {
                    // Terminal will auto-redraw
                }
                AppEvent::Tick => {
                    // Could do periodic updates here
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

async fn load_feeds(app: &mut App) -> Result<()> {
    let feed_repo = FeedRepository::new(&app.db);
    app.feeds = feed_repo.list_all().await?;

    if !app.feeds.is_empty() {
        load_articles(app).await?;
    }

    Ok(())
}

async fn load_articles(app: &mut App) -> Result<()> {
    if let Some(feed) = app.current_feed() {
        let article_repo = ArticleRepository::new(&app.db);
        let unread_only = matches!(app.view_mode, ViewMode::UnreadOnly);
        app.articles = article_repo.list_by_feed(feed.id, unread_only).await?;
        app.selected_article = 0;
        app.detail_scroll = 0;
        app.image_cache = None; // Clear image cache when articles change
    }

    Ok(())
}

/// Load image for the current article if available
async fn load_article_image(app: &mut App) {
    // Only load if image preview is enabled
    if !app.config.ui.image_preview {
        return;
    }

    if let Some(article) = app.current_article() {
        if let Some(ref url) = article.image_url {
            // Check if we already have this image cached
            if let Some(ref cache) = app.image_cache {
                if cache.url == *url {
                    return; // Already cached
                }
            }

            // Start loading the image
            let url_clone = url.clone();
            app.image_cache = Some(ImageCache::new(url_clone.clone()));

            // Load image (non-blocking would be better but for simplicity we do it inline)
            match load_image(&url_clone).await {
                Ok(img) => {
                    if let Some(ref mut cache) = app.image_cache {
                        cache.data = Some(img);
                        cache.loading = false;
                    }
                }
                Err(e) => {
                    if let Some(ref mut cache) = app.image_cache {
                        cache.error = Some(e);
                        cache.loading = false;
                    }
                }
            }
        } else {
            app.image_cache = None;
        }
    }
}

async fn handle_action(app: &mut App, action: Action, db: &Database) -> Result<()> {
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
                        load_articles(app).await?;
                    }
                }
                // Load image for the current article
                load_article_image(app).await;
            }
        }
        Action::MoveUp => {
            let prev_feed = app.selected_feed;
            app.move_up();
            if app.focus == Focus::Subscriptions && prev_feed != app.selected_feed {
                load_articles(app).await?;
            }
        }
        Action::MoveDown => {
            let prev_feed = app.selected_feed;
            app.move_down();
            if app.focus == Focus::Subscriptions && prev_feed != app.selected_feed {
                load_articles(app).await?;
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
                    // Reload to reflect change
                    load_articles(app).await?;
                }
                app.focus = Focus::ArticleDetail;
                // Load image for the current article
                load_article_image(app).await;
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
                load_articles(app).await?;
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
        }
        Action::Backspace => {
            app.search_query.pop();
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
