use std::sync::Arc;

use kenseader_core::feed::{Article, Feed};
use kenseader_core::storage::Database;
use kenseader_core::AppConfig;
use uuid::Uuid;

/// Current focus panel in the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Subscriptions,
    ArticleList,
    ArticleDetail,
}

/// View mode for article list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Show all articles
    All,
    /// Show only unread articles
    UnreadOnly,
}

/// Application mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Normal browsing mode
    Normal,
    /// Search mode (forward)
    SearchForward(String),
    /// Search mode (backward)
    SearchBackward(String),
    /// Delete confirmation
    DeleteConfirm(Uuid),
    /// Help overlay
    Help,
}

/// Application state
pub struct App {
    /// Database connection
    pub db: Arc<Database>,
    /// Application configuration
    pub config: Arc<AppConfig>,
    /// List of feeds
    pub feeds: Vec<Feed>,
    /// Currently selected feed index
    pub selected_feed: usize,
    /// Articles for the selected feed
    pub articles: Vec<Article>,
    /// Currently selected article index
    pub selected_article: usize,
    /// Current focus panel
    pub focus: Focus,
    /// Current view mode
    pub view_mode: ViewMode,
    /// Current application mode
    pub mode: Mode,
    /// Scroll offset for article detail
    pub detail_scroll: u16,
    /// Search query
    pub search_query: String,
    /// Search matches (article indices)
    pub search_matches: Vec<usize>,
    /// Current search match index
    pub current_match: usize,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Status message
    pub status_message: Option<String>,
}

impl App {
    pub fn new(db: Arc<Database>, config: Arc<AppConfig>) -> Self {
        Self {
            db,
            config,
            feeds: Vec::new(),
            selected_feed: 0,
            articles: Vec::new(),
            selected_article: 0,
            focus: Focus::Subscriptions,
            view_mode: ViewMode::All,
            mode: Mode::Normal,
            detail_scroll: 0,
            search_query: String::new(),
            search_matches: Vec::new(),
            current_match: 0,
            should_quit: false,
            status_message: None,
        }
    }

    /// Get the currently selected feed
    pub fn current_feed(&self) -> Option<&Feed> {
        self.feeds.get(self.selected_feed)
    }

    /// Get the currently selected article
    pub fn current_article(&self) -> Option<&Article> {
        self.articles.get(self.selected_article)
    }

    /// Move focus to the next panel (right)
    pub fn focus_right(&mut self) {
        self.focus = match self.focus {
            Focus::Subscriptions => Focus::ArticleList,
            Focus::ArticleList => Focus::ArticleDetail,
            Focus::ArticleDetail => Focus::ArticleDetail,
        };
    }

    /// Move focus to the previous panel (left)
    pub fn focus_left(&mut self) {
        self.focus = match self.focus {
            Focus::Subscriptions => Focus::Subscriptions,
            Focus::ArticleList => Focus::Subscriptions,
            Focus::ArticleDetail => Focus::ArticleList,
        };
    }

    /// Move selection down in the current panel
    pub fn move_down(&mut self) {
        match self.focus {
            Focus::Subscriptions => {
                if !self.feeds.is_empty() && self.selected_feed < self.feeds.len() - 1 {
                    self.selected_feed += 1;
                }
            }
            Focus::ArticleList => {
                if !self.articles.is_empty() && self.selected_article < self.articles.len() - 1 {
                    self.selected_article += 1;
                }
            }
            Focus::ArticleDetail => {
                self.detail_scroll = self.detail_scroll.saturating_add(1);
            }
        }
    }

    /// Move selection up in the current panel
    pub fn move_up(&mut self) {
        match self.focus {
            Focus::Subscriptions => {
                self.selected_feed = self.selected_feed.saturating_sub(1);
            }
            Focus::ArticleList => {
                self.selected_article = self.selected_article.saturating_sub(1);
            }
            Focus::ArticleDetail => {
                self.detail_scroll = self.detail_scroll.saturating_sub(1);
            }
        }
    }

    /// Scroll down by half page
    pub fn scroll_half_page_down(&mut self) {
        match self.focus {
            Focus::Subscriptions => {
                let jump = (self.feeds.len() / 2).max(1);
                self.selected_feed = (self.selected_feed + jump).min(self.feeds.len().saturating_sub(1));
            }
            Focus::ArticleList => {
                let jump = (self.articles.len() / 2).max(1);
                self.selected_article = (self.selected_article + jump).min(self.articles.len().saturating_sub(1));
            }
            Focus::ArticleDetail => {
                self.detail_scroll = self.detail_scroll.saturating_add(10);
            }
        }
    }

    /// Scroll up by half page
    pub fn scroll_half_page_up(&mut self) {
        match self.focus {
            Focus::Subscriptions => {
                let jump = (self.feeds.len() / 2).max(1);
                self.selected_feed = self.selected_feed.saturating_sub(jump);
            }
            Focus::ArticleList => {
                let jump = (self.articles.len() / 2).max(1);
                self.selected_article = self.selected_article.saturating_sub(jump);
            }
            Focus::ArticleDetail => {
                self.detail_scroll = self.detail_scroll.saturating_sub(10);
            }
        }
    }

    /// Jump to the beginning
    pub fn jump_to_top(&mut self) {
        match self.focus {
            Focus::Subscriptions => self.selected_feed = 0,
            Focus::ArticleList => self.selected_article = 0,
            Focus::ArticleDetail => self.detail_scroll = 0,
        }
    }

    /// Jump to the end
    pub fn jump_to_bottom(&mut self) {
        match self.focus {
            Focus::Subscriptions => {
                self.selected_feed = self.feeds.len().saturating_sub(1);
            }
            Focus::ArticleList => {
                self.selected_article = self.articles.len().saturating_sub(1);
            }
            Focus::ArticleDetail => {
                self.detail_scroll = u16::MAX; // Will be clamped during rendering
            }
        }
    }

    /// Toggle between all and unread-only view mode
    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::All => ViewMode::UnreadOnly,
            ViewMode::UnreadOnly => ViewMode::All,
        };
    }

    /// Set a status message
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
    }

    /// Clear the status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Check if we're in a mode that accepts text input
    pub fn is_input_mode(&self) -> bool {
        matches!(self.mode, Mode::SearchForward(_) | Mode::SearchBackward(_))
    }
}
