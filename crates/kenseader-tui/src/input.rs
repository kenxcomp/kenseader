use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, Focus, Mode};

/// Input action that can be performed
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Quit,
    FocusLeft,
    FocusRight,
    MoveUp,
    MoveDown,
    ScrollHalfPageDown,
    ScrollHalfPageUp,
    ScrollPageDown,
    ScrollPageUp,
    JumpToTop,
    JumpToBottom,
    PendingG,  // First 'g' press, waiting for second 'g'
    Select,
    OpenInBrowser,
    Delete,
    ToggleSaved,
    Refresh,
    StartSearchForward,
    StartSearchBackward,
    NextMatch,
    PrevMatch,
    ToggleUnreadOnly,
    ToggleRead,       // Toggle article read/unread status
    HistoryBack,      // Navigate to previous article in history
    HistoryForward,   // Navigate to next article in history
    ToggleSelect,     // Space: toggle selection and move to next
    VisualMode,       // 'v': enter/exit visual selection mode
    ClearSelection,   // Esc: clear selection
    // Image navigation and viewing
    OpenImage,        // 'o': open image in external viewer
    ViewImage,        // Enter: enter fullscreen image viewer
    NextImage,        // Tab/n: focus/navigate to next image
    PrevImage,        // Shift+Tab/p: focus/navigate to previous image
    ExitImageViewer,  // q/Esc: exit fullscreen image viewer
    // Article navigation (ArticleDetail only, respects UnreadOnly mode)
    NextArticle,      // Ctrl+J: Switch to next article
    PrevArticle,      // Ctrl+K: Switch to previous article
    ExitMode,
    Confirm,
    Cancel,
    InputChar(char),
    Backspace,
    None,
}

/// Handle a key event and return the corresponding action
pub fn handle_key_event(key: KeyEvent, app: &App) -> Action {
    // Handle input mode (search)
    if app.is_input_mode() {
        return handle_input_mode(key);
    }

    // Handle special modes
    match &app.mode {
        Mode::DeleteConfirm(_) | Mode::BatchDeleteConfirm => return handle_confirm_mode(key),
        Mode::Help => {
            // Any key exits help
            return Action::ExitMode;
        }
        Mode::ImageViewer(_) => return handle_image_viewer_mode(key),
        _ => {}
    }

    // Normal mode keybindings
    match (key.code, key.modifiers) {
        // Quit
        (KeyCode::Char('q'), KeyModifiers::NONE) => Action::Quit,
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Action::Quit,

        // Navigation between panels
        (KeyCode::Char('h'), KeyModifiers::NONE) => Action::FocusLeft,
        (KeyCode::Char('l'), KeyModifiers::NONE) => Action::FocusRight,
        (KeyCode::Left, KeyModifiers::NONE) => Action::FocusLeft,
        (KeyCode::Right, KeyModifiers::NONE) => Action::FocusRight,

        // Navigation within panel
        (KeyCode::Char('j'), KeyModifiers::NONE) => Action::MoveDown,
        (KeyCode::Char('k'), KeyModifiers::NONE) => Action::MoveUp,
        (KeyCode::Down, KeyModifiers::NONE) => Action::MoveDown,
        (KeyCode::Up, KeyModifiers::NONE) => Action::MoveUp,

        // Scrolling
        (KeyCode::Char('d'), KeyModifiers::CONTROL) => Action::ScrollHalfPageDown,
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => Action::ScrollHalfPageUp,
        (KeyCode::Char('f'), KeyModifiers::CONTROL) => Action::ScrollPageDown,
        (KeyCode::Char('b'), KeyModifiers::CONTROL) => Action::ScrollPageUp,

        // Article navigation (ArticleDetail only, respects UnreadOnly mode)
        (KeyCode::Char('j'), KeyModifiers::CONTROL) if app.focus == Focus::ArticleDetail => {
            Action::NextArticle
        }
        (KeyCode::Char('k'), KeyModifiers::CONTROL) if app.focus == Focus::ArticleDetail => {
            Action::PrevArticle
        }

        // Jump to top/bottom
        (KeyCode::Char('g'), KeyModifiers::NONE) => {
            // gg requires double press
            if app.pending_key == Some('g') {
                Action::JumpToTop
            } else {
                Action::PendingG
            }
        }
        (KeyCode::Char('G'), KeyModifiers::SHIFT) => Action::JumpToBottom,

        // Selection / Image viewer
        (KeyCode::Enter, KeyModifiers::NONE) => {
            if app.focus == Focus::ArticleDetail {
                // In ArticleDetail, Enter opens fullscreen image viewer
                Action::ViewImage
            } else {
                Action::Select
            }
        }

        // Article actions
        (KeyCode::Char('b'), KeyModifiers::NONE)
            if app.focus == Focus::ArticleDetail || app.focus == Focus::ArticleList =>
        {
            Action::OpenInBrowser
        }
        // Image navigation (ArticleDetail)
        (KeyCode::Char('o'), KeyModifiers::NONE) if app.focus == Focus::ArticleDetail => {
            Action::OpenImage
        }
        (KeyCode::Tab, KeyModifiers::NONE) if app.focus == Focus::ArticleDetail => {
            Action::NextImage
        }
        (KeyCode::BackTab, KeyModifiers::SHIFT) if app.focus == Focus::ArticleDetail => {
            Action::PrevImage
        }
        (KeyCode::Char('s'), KeyModifiers::NONE) => Action::ToggleSaved,
        (KeyCode::Char('r'), KeyModifiers::NONE) => Action::Refresh,

        // 'd' key: Delete feed in Subscriptions, Toggle read in ArticleList/Detail
        // Batch delete takes priority if feeds are selected
        (KeyCode::Char('d'), KeyModifiers::NONE) => {
            if !app.selected_feeds.is_empty() {
                Action::Delete
            } else {
                match app.focus {
                    Focus::Subscriptions => Action::Delete,
                    Focus::ArticleList | Focus::ArticleDetail => Action::ToggleRead,
                }
            }
        }

        // Search
        (KeyCode::Char('/'), KeyModifiers::NONE) => Action::StartSearchForward,
        (KeyCode::Char('?'), KeyModifiers::SHIFT) => Action::StartSearchBackward,
        (KeyCode::Char('n'), KeyModifiers::NONE) => Action::NextMatch,
        (KeyCode::Char('N'), KeyModifiers::SHIFT) => Action::PrevMatch,

        // View mode toggle
        (KeyCode::Char('i'), KeyModifiers::NONE) => Action::ToggleUnreadOnly,

        // History navigation
        (KeyCode::Char('u'), KeyModifiers::NONE) => Action::HistoryBack,
        (KeyCode::Char('r'), KeyModifiers::CONTROL) => Action::HistoryForward,

        // Selection (yazi-like)
        (KeyCode::Char(' '), KeyModifiers::NONE) => Action::ToggleSelect,
        (KeyCode::Char('v'), KeyModifiers::NONE) => Action::VisualMode,

        // Escape: clear selection if any, otherwise exit mode
        (KeyCode::Esc, KeyModifiers::NONE) => {
            if app.is_visual_mode()
                || !app.selected_articles.is_empty()
                || !app.selected_feeds.is_empty()
            {
                Action::ClearSelection
            } else {
                Action::ExitMode
            }
        }

        _ => Action::None,
    }
}

/// Handle key events in input mode (search)
fn handle_input_mode(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::Confirm,
        KeyCode::Esc => Action::Cancel,
        KeyCode::Backspace => Action::Backspace,
        KeyCode::Char(c) => Action::InputChar(c),
        _ => Action::None,
    }
}

/// Handle key events in confirmation mode
fn handle_confirm_mode(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => Action::Confirm,
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => Action::Cancel,
        _ => Action::None,
    }
}

/// Handle key events in fullscreen image viewer mode
fn handle_image_viewer_mode(key: KeyEvent) -> Action {
    match (key.code, key.modifiers) {
        // Exit image viewer
        (KeyCode::Char('q'), KeyModifiers::NONE) => Action::ExitImageViewer,
        (KeyCode::Esc, KeyModifiers::NONE) => Action::ExitImageViewer,
        // Navigate images
        (KeyCode::Char('n'), KeyModifiers::NONE) => Action::NextImage,
        (KeyCode::Char('l'), KeyModifiers::NONE) => Action::NextImage,
        (KeyCode::Right, KeyModifiers::NONE) => Action::NextImage,
        (KeyCode::Char(' '), KeyModifiers::NONE) => Action::NextImage,
        (KeyCode::Char('p'), KeyModifiers::NONE) => Action::PrevImage,
        (KeyCode::Char('h'), KeyModifiers::NONE) => Action::PrevImage,
        (KeyCode::Left, KeyModifiers::NONE) => Action::PrevImage,
        // Open in external viewer
        (KeyCode::Char('o'), KeyModifiers::NONE) => Action::OpenImage,
        (KeyCode::Enter, KeyModifiers::NONE) => Action::OpenImage,
        _ => Action::None,
    }
}
