use std::path::PathBuf;
use std::sync::Arc;

use kenseader_core::feed::{Article, Feed};
use kenseader_core::storage::Database;
use kenseader_core::AppConfig;
use uuid::Uuid;

use crate::rich_content::{ArticleImageCache, ContentElement, RichContent};

/// Rich content state for the current article
pub struct RichArticleState {
    /// Parsed content elements
    pub content: RichContent,
    /// Image cache for this article
    pub image_cache: ArticleImageCache,
    /// Pre-computed line height for each element
    pub element_heights: Vec<u16>,
    /// Total content height in lines
    pub total_height: u16,
    /// Current viewport height
    pub viewport_height: u16,
    /// Image height in terminal rows
    pub image_height: u16,
}

impl RichArticleState {
    /// Default image height in terminal rows
    const DEFAULT_IMAGE_HEIGHT: u16 = 12;

    /// Create a new RichArticleState from HTML content
    pub fn from_html(html: &str, data_dir: Option<&PathBuf>) -> Self {
        let content = RichContent::from_html(html);
        let image_cache = ArticleImageCache::new(data_dir);
        Self {
            content,
            image_cache,
            element_heights: Vec::new(),
            total_height: 0,
            viewport_height: 0,
            image_height: Self::DEFAULT_IMAGE_HEIGHT,
        }
    }

    /// Create from plain text (fallback)
    pub fn from_text(text: &str, data_dir: Option<&PathBuf>) -> Self {
        let content = RichContent::from_text(text);
        let image_cache = ArticleImageCache::new(data_dir);
        Self {
            content,
            image_cache,
            element_heights: Vec::new(),
            total_height: 0,
            viewport_height: 0,
            image_height: Self::DEFAULT_IMAGE_HEIGHT,
        }
    }

    /// Calculate heights for all elements given a width
    pub fn calculate_heights(&mut self, width: u16) {
        self.element_heights.clear();
        self.total_height = 0;

        for element in &self.content.elements {
            let height = match element {
                ContentElement::Text(text) => Self::text_height(text, width),
                ContentElement::Heading(_, text) => Self::text_height(text, width) + 1,
                ContentElement::Image { .. } => self.image_height,
                ContentElement::Quote(text) => Self::text_height(text, width.saturating_sub(2)),
                ContentElement::Code(text) => text.lines().count() as u16 + 2,
                ContentElement::ListItem(text) => Self::text_height(text, width.saturating_sub(2)),
                ContentElement::Separator => 1,
                ContentElement::EmptyLine => 1,
            };

            self.element_heights.push(height);
            self.total_height += height;
        }
    }

    /// Calculate text height with word wrapping
    fn text_height(text: &str, width: u16) -> u16 {
        if width == 0 {
            return 1;
        }
        let width = width as usize;
        let mut lines = 0u16;
        for line in text.lines() {
            if line.is_empty() {
                lines += 1;
            } else {
                lines += ((line.chars().count() + width - 1) / width) as u16;
            }
        }
        lines.max(1)
    }

    /// Get image URLs that need loading in the visible range
    pub fn get_urls_needing_load(&self, scroll: u16, viewport_height: u16) -> Vec<String> {
        let mut current_y = 0u16;
        let mut urls = Vec::new();

        for (idx, element) in self.content.elements.iter().enumerate() {
            let height = self.element_heights.get(idx).copied().unwrap_or(1);

            // Check if element is in visible range (with some margin for preloading)
            let in_range = current_y + height > scroll.saturating_sub(20)
                && current_y < scroll + viewport_height + 20;

            if in_range {
                if let ContentElement::Image { url, .. } = element {
                    if !self.image_cache.is_ready(url) && !self.image_cache.is_loading(url) {
                        urls.push(url.clone());
                    }
                }
            }

            current_y += height;
        }

        urls
    }

    /// Clear all cached data
    pub fn clear(&mut self) {
        self.image_cache.clear();
        self.element_heights.clear();
        self.total_height = 0;
    }
}

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
    /// Pending key for multi-key sequences (e.g., 'gg')
    pub pending_key: Option<char>,
    /// Rich content state for current article (replaces image_cache)
    pub rich_state: Option<RichArticleState>,
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
            pending_key: None,
            rich_state: None,
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

    /// Get the currently selected article mutably
    pub fn current_article_mut(&mut self) -> Option<&mut Article> {
        self.articles.get_mut(self.selected_article)
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

    /// Clear the pending key
    pub fn clear_pending_key(&mut self) {
        self.pending_key = None;
    }

    /// Navigate to next search match
    pub fn next_search_match(&mut self) {
        if !self.search_matches.is_empty() {
            self.current_match = (self.current_match + 1) % self.search_matches.len();
            if let Some(&idx) = self.search_matches.get(self.current_match) {
                self.selected_article = idx;
            }
        }
    }

    /// Navigate to previous search match
    pub fn prev_search_match(&mut self) {
        if !self.search_matches.is_empty() {
            self.current_match = if self.current_match == 0 {
                self.search_matches.len() - 1
            } else {
                self.current_match - 1
            };
            if let Some(&idx) = self.search_matches.get(self.current_match) {
                self.selected_article = idx;
            }
        }
    }

    /// Execute search and find matches
    pub fn execute_search(&mut self) {
        self.search_matches.clear();
        self.current_match = 0;

        if self.search_query.is_empty() {
            return;
        }

        let query = self.search_query.to_lowercase();
        for (idx, article) in self.articles.iter().enumerate() {
            let title_match = article.title.to_lowercase().contains(&query);
            let content_match = article
                .content
                .as_ref()
                .map(|c| c.to_lowercase().contains(&query))
                .unwrap_or(false);
            if title_match || content_match {
                self.search_matches.push(idx);
            }
        }

        // Navigate to first match
        if let Some(&idx) = self.search_matches.first() {
            self.selected_article = idx;
        }
    }
}
