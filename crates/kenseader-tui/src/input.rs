use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, Focus, Mode};
use crate::keymap::{KeyBinding, Keymap};

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
/// Uses the provided keymap for dynamic key binding lookup
pub fn handle_key_event(key: KeyEvent, app: &App, keymap: &Keymap) -> Action {
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
        Mode::ImageViewer(_) => return handle_image_viewer_mode(key, keymap),
        _ => {}
    }

    // Create key binding from event
    let binding = KeyBinding::new(key.code, key.modifiers);

    // Handle "gg" sequence for jump-to-top
    if keymap.has_pending_g() {
        if binding.code == KeyCode::Char('g') && binding.modifiers == KeyModifiers::NONE {
            if app.pending_key == Some('g') {
                // Second 'g' press - complete the sequence
                if let Some(action) = keymap.get_pending_g_action() {
                    return action.clone();
                }
            } else {
                // First 'g' press - start pending sequence
                return Action::PendingG;
            }
        }
    }

    // Lookup action from keymap
    if let Some(action) = keymap.get(&binding) {
        // Apply context-specific overrides
        return apply_context_overrides(action.clone(), app, &binding);
    }

    Action::None
}

/// Apply context-specific overrides to actions
/// This handles cases where the same key does different things based on focus/state
fn apply_context_overrides(action: Action, app: &App, binding: &KeyBinding) -> Action {
    match action {
        // NextArticle/PrevArticle only work in ArticleDetail
        Action::NextArticle | Action::PrevArticle => {
            if app.focus == Focus::ArticleDetail {
                action
            } else {
                Action::None
            }
        }
        // Select vs ViewImage based on focus
        Action::Select => {
            if app.focus == Focus::ArticleDetail {
                // In ArticleDetail, Enter opens fullscreen image viewer
                Action::ViewImage
            } else {
                Action::Select
            }
        }
        // ViewImage only in ArticleDetail
        Action::ViewImage => {
            if app.focus == Focus::ArticleDetail {
                Action::ViewImage
            } else {
                Action::Select
            }
        }
        // OpenInBrowser only in ArticleList or ArticleDetail
        Action::OpenInBrowser => {
            if app.focus == Focus::ArticleDetail || app.focus == Focus::ArticleList {
                Action::OpenInBrowser
            } else {
                Action::None
            }
        }
        // OpenImage only in ArticleDetail
        Action::OpenImage => {
            if app.focus == Focus::ArticleDetail {
                Action::OpenImage
            } else {
                Action::None
            }
        }
        // NextImage/PrevImage only in ArticleDetail
        Action::NextImage | Action::PrevImage => {
            if app.focus == Focus::ArticleDetail {
                action
            } else {
                Action::None
            }
        }
        // ToggleRead becomes Delete in Subscriptions, or when feeds are selected
        Action::ToggleRead => {
            if !app.selected_feeds.is_empty() {
                Action::Delete
            } else {
                match app.focus {
                    Focus::Subscriptions => Action::Delete,
                    Focus::ArticleList | Focus::ArticleDetail => Action::ToggleRead,
                }
            }
        }
        // Escape: clear selection if any, otherwise exit mode
        Action::ExitMode => {
            if binding.code == KeyCode::Esc && binding.modifiers == KeyModifiers::NONE {
                if app.is_visual_mode()
                    || !app.selected_articles.is_empty()
                    || !app.selected_feeds.is_empty()
                {
                    Action::ClearSelection
                } else {
                    Action::ExitMode
                }
            } else {
                action
            }
        }
        _ => action,
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
fn handle_image_viewer_mode(key: KeyEvent, keymap: &Keymap) -> Action {
    let binding = KeyBinding::new(key.code, key.modifiers);

    // Check for quit (always exits viewer)
    if let Some(action) = keymap.get(&binding) {
        if matches!(action, Action::Quit) {
            return Action::ExitImageViewer;
        }
    }

    // Escape always exits
    if key.code == KeyCode::Esc && key.modifiers == KeyModifiers::NONE {
        return Action::ExitImageViewer;
    }

    // Image navigation - check configured keys
    if let Some(action) = keymap.get(&binding) {
        match action {
            Action::NextImage | Action::MoveDown | Action::FocusRight => {
                return Action::NextImage;
            }
            Action::PrevImage | Action::MoveUp | Action::FocusLeft => {
                return Action::PrevImage;
            }
            Action::OpenImage | Action::Select => {
                return Action::OpenImage;
            }
            _ => {}
        }
    }

    // Hardcoded fallbacks for image viewer navigation
    match (key.code, key.modifiers) {
        // Additional navigation keys
        (KeyCode::Right, KeyModifiers::NONE) => Action::NextImage,
        (KeyCode::Left, KeyModifiers::NONE) => Action::PrevImage,
        (KeyCode::Char(' '), KeyModifiers::NONE) => Action::NextImage,
        (KeyCode::Enter, KeyModifiers::NONE) => Action::OpenImage,
        _ => Action::None,
    }
}
