//! Nord theme
//! https://www.nordtheme.com/

use ratatui::style::Color;
use crate::theme::Theme;

/// Nord default theme
pub fn default() -> Theme {
    Theme {
        // Polar Night
        bg0: Color::Rgb(0x2e, 0x34, 0x40), // nord0
        bg1: Color::Rgb(0x3b, 0x42, 0x52), // nord1
        bg2: Color::Rgb(0x43, 0x4c, 0x5e), // nord2
        bg3: Color::Rgb(0x4c, 0x56, 0x6a), // nord3
        // Snow Storm
        fg0: Color::Rgb(0xec, 0xef, 0xf4), // nord6
        fg1: Color::Rgb(0xe5, 0xe9, 0xf0), // nord5
        grey0: Color::Rgb(0x4c, 0x56, 0x6a), // nord3
        grey1: Color::Rgb(0x5e, 0x68, 0x7a), // nord3 lighter
        grey2: Color::Rgb(0xd8, 0xde, 0xe9), // nord4
        // Frost
        aqua: Color::Rgb(0x8f, 0xbc, 0xbb),   // nord7
        blue: Color::Rgb(0x88, 0xc0, 0xd0),   // nord8
        purple: Color::Rgb(0x81, 0xa1, 0xc1), // nord9
        // Aurora
        red: Color::Rgb(0xbf, 0x61, 0x6a),    // nord11
        orange: Color::Rgb(0xd0, 0x87, 0x70), // nord12
        yellow: Color::Rgb(0xeb, 0xcb, 0x8b), // nord13
        green: Color::Rgb(0xa3, 0xbe, 0x8c),  // nord14
        selection: Color::Rgb(0x43, 0x4c, 0x5e),
        unread: Color::Rgb(0xeb, 0xcb, 0x8b),
        read: Color::Rgb(0x5e, 0x68, 0x7a),
        error: Color::Rgb(0xbf, 0x61, 0x6a),
        success: Color::Rgb(0xa3, 0xbe, 0x8c),
        warning: Color::Rgb(0xd0, 0x87, 0x70),
        info: Color::Rgb(0x88, 0xc0, 0xd0),
        accent: Color::Rgb(0x8f, 0xbc, 0xbb),
    }
}
