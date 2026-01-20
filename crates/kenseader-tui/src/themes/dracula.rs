//! Dracula theme
//! https://draculatheme.com/

use ratatui::style::Color;
use crate::theme::Theme;

/// Dracula default theme
pub fn default() -> Theme {
    Theme {
        bg0: Color::Rgb(0x28, 0x2a, 0x36), // Background
        bg1: Color::Rgb(0x21, 0x22, 0x2c), // Current Line (darker)
        bg2: Color::Rgb(0x44, 0x47, 0x5a), // Selection
        bg3: Color::Rgb(0x62, 0x72, 0xa4),     // Comment
        fg0: Color::Rgb(0xf8, 0xf8, 0xf2), // Foreground
        fg1: Color::Rgb(0xe9, 0xe9, 0xea), // Foreground (slightly dimmer)
        grey0: Color::Rgb(0x62, 0x72, 0xa4),   // Comment
        grey1: Color::Rgb(0x5a, 0x5c, 0x6d), // Darker gray
        grey2: Color::Rgb(0x7a, 0x7c, 0x8d), // Lighter gray
        red: Color::Rgb(0xff, 0x55, 0x55),    // Red
        orange: Color::Rgb(0xff, 0xb8, 0x6c), // Orange
        yellow: Color::Rgb(0xf1, 0xfa, 0x8c), // Yellow
        green: Color::Rgb(0x50, 0xfa, 0x7b),  // Green
        aqua: Color::Rgb(0x8b, 0xe9, 0xfd),   // Cyan
        blue: Color::Rgb(0xbd, 0x93, 0xf9),   // Purple (used as blue)
        purple: Color::Rgb(0xff, 0x79, 0xc6), // Pink
        selection: Color::Rgb(0x44, 0x47, 0x5a),
        unread: Color::Rgb(0xf1, 0xfa, 0x8c),
        read: Color::Rgb(0x62, 0x72, 0xa4),
        error: Color::Rgb(0xff, 0x55, 0x55),
        success: Color::Rgb(0x50, 0xfa, 0x7b),
        warning: Color::Rgb(0xff, 0xb8, 0x6c),
        info: Color::Rgb(0x8b, 0xe9, 0xfd),
        accent: Color::Rgb(0xbd, 0x93, 0xf9),
    }
}
