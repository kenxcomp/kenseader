//! Monokai theme
//! https://monokai.pro/

use ratatui::style::Color;
use crate::theme::Theme;

/// Monokai default theme
pub fn default() -> Theme {
    Theme {
        bg0: Color::Rgb(0x27, 0x28, 0x22), // background
        bg1: Color::Rgb(0x3e, 0x3d, 0x32), // selection
        bg2: Color::Rgb(0x49, 0x48, 0x3e), // line
        bg3: Color::Rgb(0x75, 0x71, 0x5e), // comment
        fg0: Color::Rgb(0xf8, 0xf8, 0xf2), // foreground
        fg1: Color::Rgb(0xd0, 0xd0, 0xc0), // foreground (dimmer)
        grey0: Color::Rgb(0x75, 0x71, 0x5e), // comment
        grey1: Color::Rgb(0x5f, 0x5c, 0x4d), // darker gray
        grey2: Color::Rgb(0x90, 0x8c, 0x77), // lighter gray
        red: Color::Rgb(0xf9, 0x26, 0x72),    // magenta/red
        orange: Color::Rgb(0xfd, 0x97, 0x1f), // orange
        yellow: Color::Rgb(0xe6, 0xdb, 0x74), // yellow
        green: Color::Rgb(0xa6, 0xe2, 0x2e),  // green
        aqua: Color::Rgb(0x66, 0xd9, 0xef),   // cyan
        blue: Color::Rgb(0x66, 0xd9, 0xef),   // cyan (monokai uses cyan as blue)
        purple: Color::Rgb(0xae, 0x81, 0xff), // purple
        selection: Color::Rgb(0x3e, 0x3d, 0x32),
        unread: Color::Rgb(0xe6, 0xdb, 0x74),
        read: Color::Rgb(0x75, 0x71, 0x5e),
        error: Color::Rgb(0xf9, 0x26, 0x72),
        success: Color::Rgb(0xa6, 0xe2, 0x2e),
        warning: Color::Rgb(0xfd, 0x97, 0x1f),
        info: Color::Rgb(0x66, 0xd9, 0xef),
        accent: Color::Rgb(0xae, 0x81, 0xff),
    }
}
