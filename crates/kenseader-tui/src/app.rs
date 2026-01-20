use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use kenseader_core::feed::{Article, Feed};
use kenseader_core::ipc::DaemonClient;
use kenseader_core::AppConfig;
use uuid::Uuid;

use crate::image_renderer::ImageRenderer;
use crate::rich_content::{ArticleImageCache, ContentElement, FocusableItem, PreloadCache, ResizedImageCache, RichContent};
use crate::theme::Theme;

/// Rich content state for the current article
pub struct RichArticleState {
    /// Parsed content elements
    pub content: RichContent,
    /// Image cache for this article
    pub image_cache: ArticleImageCache,
    /// Pre-resized image cache for halfblock rendering (avoids resize on every frame)
    pub resized_cache: ResizedImageCache,
    /// Pre-computed line height for each element
    pub element_heights: Vec<u16>,
    /// Total content height in lines
    pub total_height: u16,
    /// Current viewport height
    pub viewport_height: u16,
    /// Image height in terminal rows
    pub image_height: u16,
    /// Index of currently focused item in focusable_items (images + links)
    pub focused_item: Option<usize>,
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
            resized_cache: ResizedImageCache::new(),
            element_heights: Vec::new(),
            total_height: 0,
            viewport_height: 0,
            image_height: Self::DEFAULT_IMAGE_HEIGHT,
            focused_item: None,
        }
    }

    /// Create from plain text (fallback)
    pub fn from_text(text: &str, data_dir: Option<&PathBuf>) -> Self {
        let content = RichContent::from_text(text);
        let image_cache = ArticleImageCache::new(data_dir);
        Self {
            content,
            image_cache,
            resized_cache: ResizedImageCache::new(),
            element_heights: Vec::new(),
            total_height: 0,
            viewport_height: 0,
            image_height: Self::DEFAULT_IMAGE_HEIGHT,
            focused_item: None,
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
        self.resized_cache.clear();
        self.element_heights.clear();
        self.total_height = 0;
        self.focused_item = None;
    }

    /// Get the currently focused item
    pub fn get_focused_item(&self) -> Option<&FocusableItem> {
        self.focused_item.and_then(|idx| self.content.focusable_items.get(idx))
    }

    /// Get focused image index (for backwards compatibility with image cache)
    pub fn focused_image_index(&self) -> Option<usize> {
        match self.get_focused_item() {
            Some(FocusableItem::Image { url_index }) => Some(*url_index),
            _ => None,
        }
    }

    /// Get focused link URL
    pub fn focused_link_url(&self) -> Option<&str> {
        match self.get_focused_item() {
            Some(FocusableItem::Link { url, .. }) => Some(url.as_str()),
            _ => None,
        }
    }

    /// Check if current focus is on an image
    pub fn is_image_focused(&self) -> bool {
        matches!(self.get_focused_item(), Some(FocusableItem::Image { .. }))
    }

    /// Check if current focus is on a link
    pub fn is_link_focused(&self) -> bool {
        matches!(self.get_focused_item(), Some(FocusableItem::Link { .. }))
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
    /// Delete confirmation (single feed)
    DeleteConfirm(Uuid),
    /// Batch delete confirmation (multiple feeds)
    BatchDeleteConfirm,
    /// Help overlay
    Help,
    /// Fullscreen image viewer mode (image index)
    ImageViewer(usize),
}

/// Application state
pub struct App {
    /// Daemon client for IPC communication (None in read-mode)
    pub client: Option<Arc<DaemonClient>>,
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
    /// Reading history stack - stores (feed_id, article_id) tuples
    /// Using IDs instead of indices to ensure correct navigation in unread-only mode
    pub read_history: Vec<(Uuid, Uuid)>,
    /// Current position in history (1-indexed, 0 means empty)
    pub history_position: usize,
    /// Selected article indices (for batch operations)
    pub selected_articles: HashSet<usize>,
    /// Selected feed indices (for batch operations)
    pub selected_feeds: HashSet<usize>,
    /// Visual mode start position for articles (None = not in visual mode)
    pub visual_start_article: Option<usize>,
    /// Visual mode start position for feeds (None = not in visual mode)
    pub visual_start_feed: Option<usize>,
    /// Whether a refresh operation is in progress
    pub is_refreshing: bool,
    /// Image renderer for high-resolution image display
    pub image_renderer: ImageRenderer,
    /// Terminal viewport height for adaptive scroll calculations
    pub viewport_height: u16,
    /// Current spinner animation frame
    pub spinner_frame: usize,
    /// Global preload cache for prefetching images before entering article detail
    pub preload_cache: PreloadCache,
    /// Read-mode: TUI reads directly from data_dir without daemon
    /// Disables refresh, feed add/delete; allows read status toggle with retry
    pub read_mode: bool,
    /// Current color theme
    pub theme: Theme,
}

/// Spinner animation frames (braille pattern)
pub const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

impl App {
    /// Create a new App with daemon client (normal mode)
    pub fn new(client: Arc<DaemonClient>, config: Arc<AppConfig>, theme: Theme) -> Self {
        Self::new_with_mode(Some(client), config, false, theme)
    }

    /// Create a new App in read-mode (direct database access, no daemon)
    pub fn new_read_mode(config: Arc<AppConfig>, theme: Theme) -> Self {
        Self::new_with_mode(None, config, true, theme)
    }

    /// Internal constructor with mode selection
    fn new_with_mode(client: Option<Arc<DaemonClient>>, config: Arc<AppConfig>, read_mode: bool, theme: Theme) -> Self {
        Self {
            client,
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
            read_history: Vec::new(),
            history_position: 0,
            selected_articles: HashSet::new(),
            selected_feeds: HashSet::new(),
            visual_start_article: None,
            visual_start_feed: None,
            is_refreshing: false,
            image_renderer: ImageRenderer::new(),
            viewport_height: 24, // Default, will be updated on first render
            spinner_frame: 0,
            preload_cache: PreloadCache::new(None), // Initialized without disk cache, will be set later
            read_mode,
            theme,
        }
    }

    /// Initialize preload cache with data directory for disk cache support
    pub fn init_preload_cache(&mut self, data_dir: Option<&PathBuf>) {
        self.preload_cache = PreloadCache::new(data_dir);
    }

    /// Get the range of article indices to preload images for
    /// Returns indices for current ± preload_count articles
    pub fn get_preload_article_range(&self, preload_count: usize) -> std::ops::Range<usize> {
        let start = self.selected_article.saturating_sub(preload_count);
        let end = (self.selected_article + preload_count + 1).min(self.articles.len());
        start..end
    }

    /// Get all image URLs for an article (cover image first, then content images)
    pub fn get_article_image_urls(article: &Article) -> Vec<String> {
        let mut urls = Vec::new();
        // Cover image has priority
        if let Some(ref cover) = article.image_url {
            if !cover.is_empty() {
                urls.push(cover.clone());
            }
        }
        // Content images
        if let Some(ref content) = article.content {
            urls.extend(RichContent::extract_image_urls(content));
        }
        urls
    }

    /// Tick the spinner animation (call on each tick event)
    pub fn tick_spinner(&mut self) {
        if self.is_refreshing {
            self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
        }
    }

    /// Get the current spinner character
    pub fn current_spinner(&self) -> char {
        SPINNER_FRAMES[self.spinner_frame]
    }

    /// Get the currently selected feed
    pub fn current_feed(&self) -> Option<&Feed> {
        self.feeds.get(self.selected_feed)
    }

    /// Get the currently selected feed mutably
    pub fn current_feed_mut(&mut self) -> Option<&mut Feed> {
        self.feeds.get_mut(self.selected_feed)
    }

    /// Get feeds to display based on view mode
    /// In UnreadOnly mode, feeds with errors are always shown (highlighted in red)
    pub fn visible_feeds(&self) -> Vec<&Feed> {
        match self.view_mode {
            ViewMode::All => self.feeds.iter().collect(),
            ViewMode::UnreadOnly => self
                .feeds
                .iter()
                .filter(|f| f.unread_count > 0 || f.has_error())
                .collect(),
        }
    }

    /// Get the actual feed index from visible index
    pub fn visible_to_actual_feed_index(&self, visible_idx: usize) -> Option<usize> {
        let visible_feeds = self.visible_feeds();
        visible_feeds
            .get(visible_idx)
            .and_then(|vf| self.feeds.iter().position(|f| f.id == vf.id))
    }

    /// Get the visible index from actual feed index
    pub fn actual_to_visible_feed_index(&self, actual_idx: usize) -> Option<usize> {
        let feed = self.feeds.get(actual_idx)?;
        self.visible_feeds().iter().position(|f| f.id == feed.id)
    }

    /// Get the currently selected article
    pub fn current_article(&self) -> Option<&Article> {
        self.articles.get(self.selected_article)
    }

    /// Get the currently selected article mutably
    pub fn current_article_mut(&mut self) -> Option<&mut Article> {
        self.articles.get_mut(self.selected_article)
    }

    /// Clear rich state and any rendered images
    /// This should be called when switching articles to avoid ghost images
    pub fn clear_rich_state(&mut self) {
        self.rich_state = None;
        self.image_renderer.clear_all();
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

    /// Find index of next unread article after current position
    /// Returns None if no unread article exists after current
    pub fn find_next_unread_article(&self) -> Option<usize> {
        for i in (self.selected_article + 1)..self.articles.len() {
            if !self.articles[i].is_read {
                return Some(i);
            }
        }
        None
    }

    /// Find index of previous unread article before current position
    /// Returns None if no unread article exists before current
    pub fn find_prev_unread_article(&self) -> Option<usize> {
        for i in (0..self.selected_article).rev() {
            if !self.articles[i].is_read {
                return Some(i);
            }
        }
        None
    }

    /// Scroll down by half page (uses viewport_height for adaptive scroll)
    pub fn scroll_half_page_down(&mut self) {
        let half_page = (self.viewport_height / 2).max(1);
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
                self.detail_scroll = self.detail_scroll.saturating_add(half_page);
            }
        }
    }

    /// Scroll up by half page (uses viewport_height for adaptive scroll)
    pub fn scroll_half_page_up(&mut self) {
        let half_page = (self.viewport_height / 2).max(1);
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
                self.detail_scroll = self.detail_scroll.saturating_sub(half_page);
            }
        }
    }

    /// Scroll down by full page (uses viewport_height for adaptive scroll)
    pub fn scroll_full_page_down(&mut self) {
        let full_page = self.viewport_height.max(1);
        match self.focus {
            Focus::Subscriptions => {
                let jump = self.feeds.len().max(1);
                self.selected_feed = (self.selected_feed + jump).min(self.feeds.len().saturating_sub(1));
            }
            Focus::ArticleList => {
                let jump = self.articles.len().max(1);
                self.selected_article = (self.selected_article + jump).min(self.articles.len().saturating_sub(1));
            }
            Focus::ArticleDetail => {
                self.detail_scroll = self.detail_scroll.saturating_add(full_page);
            }
        }
    }

    /// Scroll up by full page (uses viewport_height for adaptive scroll)
    pub fn scroll_full_page_up(&mut self) {
        let full_page = self.viewport_height.max(1);
        match self.focus {
            Focus::Subscriptions => {
                let jump = self.feeds.len().max(1);
                self.selected_feed = self.selected_feed.saturating_sub(jump);
            }
            Focus::ArticleList => {
                let jump = self.articles.len().max(1);
                self.selected_article = self.selected_article.saturating_sub(jump);
            }
            Focus::ArticleDetail => {
                self.detail_scroll = self.detail_scroll.saturating_sub(full_page);
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

    /// Record current position to history using IDs (not indices)
    pub fn push_history(&mut self) {
        // Get current feed and article IDs
        let feed_id = match self.feeds.get(self.selected_feed) {
            Some(feed) => feed.id,
            None => return,
        };
        let article_id = match self.articles.get(self.selected_article) {
            Some(article) => article.id,
            None => return,
        };

        // If not at the end of history, truncate forward history
        if self.history_position < self.read_history.len() {
            self.read_history.truncate(self.history_position);
        }

        let entry = (feed_id, article_id);

        // Avoid recording consecutive duplicates
        if self.read_history.last() != Some(&entry) {
            self.read_history.push(entry);
            self.history_position = self.read_history.len();
        }
    }

    /// Navigate back in history - returns (feed_id, article_id)
    pub fn history_back(&mut self) -> Option<(Uuid, Uuid)> {
        if self.history_position > 1 {
            self.history_position -= 1;
            self.read_history.get(self.history_position - 1).copied()
        } else {
            None
        }
    }

    /// Navigate forward in history - returns (feed_id, article_id)
    pub fn history_forward(&mut self) -> Option<(Uuid, Uuid)> {
        if self.history_position < self.read_history.len() {
            self.history_position += 1;
            self.read_history.get(self.history_position - 1).copied()
        } else {
            None
        }
    }

    /// Find feed index by ID
    pub fn find_feed_index(&self, feed_id: Uuid) -> Option<usize> {
        self.feeds.iter().position(|f| f.id == feed_id)
    }

    /// Find article index by ID
    pub fn find_article_index(&self, article_id: Uuid) -> Option<usize> {
        self.articles.iter().position(|a| a.id == article_id)
    }

    // ========== Selection Methods ==========

    /// Toggle article selection at given index
    pub fn toggle_article_selection(&mut self, index: usize) {
        if self.selected_articles.contains(&index) {
            self.selected_articles.remove(&index);
        } else {
            self.selected_articles.insert(index);
        }
    }

    /// Toggle feed selection at given index
    pub fn toggle_feed_selection(&mut self, index: usize) {
        if self.selected_feeds.contains(&index) {
            self.selected_feeds.remove(&index);
        } else {
            self.selected_feeds.insert(index);
        }
    }

    /// Clear article selection and exit visual mode
    pub fn clear_article_selection(&mut self) {
        self.selected_articles.clear();
        self.visual_start_article = None;
    }

    /// Clear feed selection and exit visual mode
    pub fn clear_feed_selection(&mut self) {
        self.selected_feeds.clear();
        self.visual_start_feed = None;
    }

    /// Update visual selection range for articles
    pub fn update_visual_selection_articles(&mut self) {
        if let Some(start) = self.visual_start_article {
            let end = self.selected_article;
            let (from, to) = if start <= end {
                (start, end)
            } else {
                (end, start)
            };
            self.selected_articles.clear();
            for i in from..=to {
                self.selected_articles.insert(i);
            }
        }
    }

    /// Update visual selection range for feeds
    pub fn update_visual_selection_feeds(&mut self) {
        if let Some(start) = self.visual_start_feed {
            let end = self.selected_feed;
            let (from, to) = if start <= end {
                (start, end)
            } else {
                (end, start)
            };
            self.selected_feeds.clear();
            for i in from..=to {
                self.selected_feeds.insert(i);
            }
        }
    }

    /// Check if currently in visual mode
    pub fn is_visual_mode(&self) -> bool {
        self.visual_start_article.is_some() || self.visual_start_feed.is_some()
    }

    /// Check if in visual mode for articles
    pub fn is_visual_mode_articles(&self) -> bool {
        self.visual_start_article.is_some()
    }

    /// Check if in visual mode for feeds
    pub fn is_visual_mode_feeds(&self) -> bool {
        self.visual_start_feed.is_some()
    }
}
