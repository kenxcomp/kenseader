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
        Mode::DeleteConfirm(_) => return handle_confirm_mode(key),
        Mode::Help => {
            // Any key exits help
            return Action::ExitMode;
        }
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

        // Selection
        (KeyCode::Enter, KeyModifiers::NONE) => Action::Select,

        // Article actions
        (KeyCode::Char('b'), KeyModifiers::NONE) if app.focus == Focus::ArticleDetail => {
            Action::OpenInBrowser
        }
        (KeyCode::Char('s'), KeyModifiers::NONE) => Action::ToggleSaved,
        (KeyCode::Char('r'), KeyModifiers::NONE) => Action::Refresh,

        // Delete (subscriptions only)
        (KeyCode::Char('d'), KeyModifiers::NONE) if app.focus == Focus::Subscriptions => {
            Action::Delete
        }

        // Search
        (KeyCode::Char('/'), KeyModifiers::NONE) => Action::StartSearchForward,
        (KeyCode::Char('?'), KeyModifiers::SHIFT) => Action::StartSearchBackward,
        (KeyCode::Char('n'), KeyModifiers::NONE) => Action::NextMatch,
        (KeyCode::Char('N'), KeyModifiers::SHIFT) => Action::PrevMatch,

        // View mode toggle
        (KeyCode::Char('i'), KeyModifiers::NONE) => Action::ToggleUnreadOnly,
        (KeyCode::Esc, KeyModifiers::NONE) => Action::ExitMode,

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
