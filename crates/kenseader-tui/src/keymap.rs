use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyModifiers};
use kenseader_core::config::KeymapConfig;
use tracing::warn;

use crate::input::Action;

/// Parsed key binding (key code + modifiers)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    pub fn simple(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::NONE)
    }

    pub fn ctrl(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::CONTROL)
    }

    pub fn shift(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::SHIFT)
    }
}

/// Runtime keymap for efficient key-to-action lookup
pub struct Keymap {
    /// Primary key bindings
    bindings: HashMap<KeyBinding, Action>,
    /// Special handling for multi-key sequences (e.g., "gg")
    /// Stores the first key and the action it triggers
    pending_g_action: Option<Action>,
}

impl Default for Keymap {
    fn default() -> Self {
        Self::from_config(&KeymapConfig::default())
    }
}

impl Keymap {
    /// Create a keymap from configuration
    pub fn from_config(config: &KeymapConfig) -> Self {
        let mut bindings = HashMap::new();
        let mut pending_g_action = None;

        // Helper to add binding with conflict detection
        let mut add_binding = |key_str: &str, action: Action| {
            // Handle special "gg" sequence
            if key_str == "gg" {
                pending_g_action = Some(action);
                return;
            }

            if let Some(binding) = parse_key_binding(key_str) {
                if let Some(existing) = bindings.get(&binding) {
                    warn!(
                        "Key conflict: '{}' already bound to {:?}, ignoring binding to {:?}",
                        key_str, existing, action
                    );
                } else {
                    bindings.insert(binding, action);
                }
            } else {
                warn!("Invalid key binding: '{}', using default", key_str);
            }
        };

        // Add all configured bindings
        add_binding(&config.quit, Action::Quit);
        add_binding(&config.focus_left, Action::FocusLeft);
        add_binding(&config.focus_right, Action::FocusRight);
        add_binding(&config.move_down, Action::MoveDown);
        add_binding(&config.move_up, Action::MoveUp);
        add_binding(&config.scroll_half_down, Action::ScrollHalfPageDown);
        add_binding(&config.scroll_half_up, Action::ScrollHalfPageUp);
        add_binding(&config.scroll_page_down, Action::ScrollPageDown);
        add_binding(&config.scroll_page_up, Action::ScrollPageUp);
        add_binding(&config.next_article, Action::NextArticle);
        add_binding(&config.prev_article, Action::PrevArticle);
        add_binding(&config.jump_to_top, Action::JumpToTop);
        add_binding(&config.jump_to_bottom, Action::JumpToBottom);
        add_binding(&config.select, Action::Select);
        add_binding(&config.open_browser, Action::OpenInBrowser);
        add_binding(&config.toggle_saved, Action::ToggleSaved);
        add_binding(&config.refresh, Action::Refresh);
        add_binding(&config.toggle_read, Action::ToggleRead);
        add_binding(&config.search_forward, Action::StartSearchForward);
        add_binding(&config.search_backward, Action::StartSearchBackward);
        add_binding(&config.next_match, Action::NextMatch);
        add_binding(&config.prev_match, Action::PrevMatch);
        add_binding(&config.toggle_unread_only, Action::ToggleUnreadOnly);
        add_binding(&config.history_back, Action::HistoryBack);
        add_binding(&config.history_forward, Action::HistoryForward);
        add_binding(&config.toggle_select, Action::ToggleSelect);
        add_binding(&config.visual_mode, Action::VisualMode);
        add_binding(&config.open_item, Action::OpenImage);
        add_binding(&config.view_image, Action::ViewImage);
        add_binding(&config.next_item, Action::NextImage);
        add_binding(&config.prev_item, Action::PrevImage);

        // Add hardcoded bindings that shouldn't be configurable
        // Ctrl+C always quits
        bindings.insert(KeyBinding::ctrl(KeyCode::Char('c')), Action::Quit);
        // Arrow keys for navigation (always available as alternatives)
        bindings.entry(KeyBinding::simple(KeyCode::Left)).or_insert(Action::FocusLeft);
        bindings.entry(KeyBinding::simple(KeyCode::Right)).or_insert(Action::FocusRight);
        bindings.entry(KeyBinding::simple(KeyCode::Up)).or_insert(Action::MoveUp);
        bindings.entry(KeyBinding::simple(KeyCode::Down)).or_insert(Action::MoveDown);
        // Escape for exiting modes/clearing selection
        bindings.insert(KeyBinding::simple(KeyCode::Esc), Action::ExitMode);

        Self {
            bindings,
            pending_g_action,
        }
    }

    /// Get action for a key binding
    pub fn get(&self, binding: &KeyBinding) -> Option<&Action> {
        self.bindings.get(binding)
    }

    /// Check if "gg" sequence is configured
    pub fn has_pending_g(&self) -> bool {
        self.pending_g_action.is_some()
    }

    /// Get the action for completed "gg" sequence
    pub fn get_pending_g_action(&self) -> Option<&Action> {
        self.pending_g_action.as_ref()
    }

    /// Check if a single 'g' press should start a pending sequence
    pub fn is_g_prefix(&self, binding: &KeyBinding) -> bool {
        self.pending_g_action.is_some()
            && binding.code == KeyCode::Char('g')
            && binding.modifiers == KeyModifiers::NONE
    }
}

/// Parse Vim-style key notation into KeyBinding
/// Supported formats:
/// - Single char: "j", "k", "h", "l", "q", etc.
/// - Uppercase (Shift): "G", "N", etc.
/// - Special chars: "/", "?", etc.
/// - Ctrl: "<C-j>", "<C-k>", etc.
/// - Shift: "<S-Tab>", "<S-g>", etc.
/// - Special keys: "<CR>", "<Enter>", "<Esc>", "<Tab>", "<Space>", "<Left>", "<Right>", "<Up>", "<Down>"
pub fn parse_key_binding(s: &str) -> Option<KeyBinding> {
    let s = s.trim();

    // Handle special notation <...>
    if s.starts_with('<') && s.ends_with('>') {
        let inner = &s[1..s.len() - 1];
        return parse_special_key(inner);
    }

    // Single character
    if s.len() == 1 {
        let c = s.chars().next()?;
        // Uppercase letters are Shift+lowercase
        if c.is_ascii_uppercase() {
            return Some(KeyBinding::shift(KeyCode::Char(c)));
        }
        return Some(KeyBinding::simple(KeyCode::Char(c)));
    }

    // "gg" is handled specially by Keymap, not here
    if s == "gg" {
        // Return 'g' binding, the double-press logic is handled elsewhere
        return Some(KeyBinding::simple(KeyCode::Char('g')));
    }

    None
}

/// Parse special key notation (content inside <...>)
fn parse_special_key(inner: &str) -> Option<KeyBinding> {
    // Handle modifiers: C- (Ctrl), S- (Shift), A-/M- (Alt)
    if let Some(rest) = inner.strip_prefix("C-") {
        let key = parse_key_name(rest)?;
        return Some(KeyBinding::ctrl(key));
    }

    if let Some(rest) = inner.strip_prefix("S-") {
        let key = parse_key_name(rest)?;
        return Some(KeyBinding::shift(key));
    }

    // Handle special key names without modifiers
    parse_key_name(inner).map(KeyBinding::simple)
}

/// Parse a key name (without modifiers)
fn parse_key_name(name: &str) -> Option<KeyCode> {
    match name.to_lowercase().as_str() {
        "cr" | "enter" | "return" => Some(KeyCode::Enter),
        "esc" | "escape" => Some(KeyCode::Esc),
        "tab" => Some(KeyCode::Tab),
        "backtab" => Some(KeyCode::BackTab),
        "space" | "spc" => Some(KeyCode::Char(' ')),
        "bs" | "backspace" => Some(KeyCode::Backspace),
        "del" | "delete" => Some(KeyCode::Delete),
        "left" => Some(KeyCode::Left),
        "right" => Some(KeyCode::Right),
        "up" => Some(KeyCode::Up),
        "down" => Some(KeyCode::Down),
        "home" => Some(KeyCode::Home),
        "end" => Some(KeyCode::End),
        "pageup" | "pgup" => Some(KeyCode::PageUp),
        "pagedown" | "pgdn" => Some(KeyCode::PageDown),
        "insert" | "ins" => Some(KeyCode::Insert),
        "f1" => Some(KeyCode::F(1)),
        "f2" => Some(KeyCode::F(2)),
        "f3" => Some(KeyCode::F(3)),
        "f4" => Some(KeyCode::F(4)),
        "f5" => Some(KeyCode::F(5)),
        "f6" => Some(KeyCode::F(6)),
        "f7" => Some(KeyCode::F(7)),
        "f8" => Some(KeyCode::F(8)),
        "f9" => Some(KeyCode::F(9)),
        "f10" => Some(KeyCode::F(10)),
        "f11" => Some(KeyCode::F(11)),
        "f12" => Some(KeyCode::F(12)),
        _ => {
            // Single character after modifier (e.g., "j" in "<C-j>")
            if name.len() == 1 {
                let c = name.chars().next()?;
                Some(KeyCode::Char(c.to_ascii_lowercase()))
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_keys() {
        assert_eq!(
            parse_key_binding("j"),
            Some(KeyBinding::simple(KeyCode::Char('j')))
        );
        assert_eq!(
            parse_key_binding("k"),
            Some(KeyBinding::simple(KeyCode::Char('k')))
        );
        assert_eq!(
            parse_key_binding("/"),
            Some(KeyBinding::simple(KeyCode::Char('/')))
        );
    }

    #[test]
    fn test_parse_uppercase_keys() {
        assert_eq!(
            parse_key_binding("G"),
            Some(KeyBinding::shift(KeyCode::Char('G')))
        );
        assert_eq!(
            parse_key_binding("N"),
            Some(KeyBinding::shift(KeyCode::Char('N')))
        );
    }

    #[test]
    fn test_parse_ctrl_keys() {
        assert_eq!(
            parse_key_binding("<C-j>"),
            Some(KeyBinding::ctrl(KeyCode::Char('j')))
        );
        assert_eq!(
            parse_key_binding("<C-d>"),
            Some(KeyBinding::ctrl(KeyCode::Char('d')))
        );
    }

    #[test]
    fn test_parse_special_keys() {
        assert_eq!(
            parse_key_binding("<CR>"),
            Some(KeyBinding::simple(KeyCode::Enter))
        );
        assert_eq!(
            parse_key_binding("<Enter>"),
            Some(KeyBinding::simple(KeyCode::Enter))
        );
        assert_eq!(
            parse_key_binding("<Esc>"),
            Some(KeyBinding::simple(KeyCode::Esc))
        );
        assert_eq!(
            parse_key_binding("<Tab>"),
            Some(KeyBinding::simple(KeyCode::Tab))
        );
        assert_eq!(
            parse_key_binding("<Space>"),
            Some(KeyBinding::simple(KeyCode::Char(' ')))
        );
        assert_eq!(
            parse_key_binding("<S-Tab>"),
            Some(KeyBinding::shift(KeyCode::Tab))
        );
    }

    #[test]
    fn test_keymap_from_config() {
        let config = KeymapConfig::default();
        let keymap = Keymap::from_config(&config);

        // Check some default bindings
        assert_eq!(
            keymap.get(&KeyBinding::simple(KeyCode::Char('q'))),
            Some(&Action::Quit)
        );
        assert_eq!(
            keymap.get(&KeyBinding::simple(KeyCode::Char('j'))),
            Some(&Action::MoveDown)
        );
        assert_eq!(
            keymap.get(&KeyBinding::ctrl(KeyCode::Char('d'))),
            Some(&Action::ScrollHalfPageDown)
        );

        // Check gg handling
        assert!(keymap.has_pending_g());
        assert_eq!(keymap.get_pending_g_action(), Some(&Action::JumpToTop));
    }
}
