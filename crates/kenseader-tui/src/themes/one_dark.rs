//! One Dark theme
//! https://github.com/atom/atom/tree/master/packages/one-dark-syntax

use ratatui::style::Color;
use crate::theme::Theme;

/// One Dark default theme
pub fn default() -> Theme {
    Theme {
        bg0: Color::Rgb(0x28, 0x2c, 0x34), // bg
        bg1: Color::Rgb(0x21, 0x25, 0x2b), // bg-darker
        bg2: Color::Rgb(0x3e, 0x44, 0x51), // bg-highlight
        bg3: Color::Rgb(0x4b, 0x52, 0x63), // gutter
        fg0: Color::Rgb(0xab, 0xb2, 0xbf), // fg
        fg1: Color::Rgb(0x9d, 0xa5, 0xb4), // fg-dim
        grey0: Color::Rgb(0x5c, 0x63, 0x70), // comment
        grey1: Color::Rgb(0x4b, 0x52, 0x63), // gutter
        grey2: Color::Rgb(0x7f, 0x84, 0x8e), // lighter gray
        red: Color::Rgb(0xe0, 0x6c, 0x75),    // red
        orange: Color::Rgb(0xd1, 0x9a, 0x66), // orange
        yellow: Color::Rgb(0xe5, 0xc0, 0x7b), // yellow
        green: Color::Rgb(0x98, 0xc3, 0x79),  // green
        aqua: Color::Rgb(0x56, 0xb6, 0xc2),   // cyan
        blue: Color::Rgb(0x61, 0xaf, 0xef),   // blue
        purple: Color::Rgb(0xc6, 0x78, 0xdd), // purple
        selection: Color::Rgb(0x3e, 0x44, 0x51),
        unread: Color::Rgb(0xe5, 0xc0, 0x7b),
        read: Color::Rgb(0x5c, 0x63, 0x70),
        error: Color::Rgb(0xe0, 0x6c, 0x75),
        success: Color::Rgb(0x98, 0xc3, 0x79),
        warning: Color::Rgb(0xd1, 0x9a, 0x66),
        info: Color::Rgb(0x61, 0xaf, 0xef),
        accent: Color::Rgb(0x56, 0xb6, 0xc2),
    }
}
