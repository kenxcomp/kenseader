use ratatui::style::Color;

/// Runtime theme with configurable colors
#[derive(Debug, Clone)]
pub struct Theme {
    // Background colors
    pub bg0: Color,
    pub bg1: Color,
    pub bg2: Color,
    pub bg3: Color,

    // Foreground colors
    pub fg0: Color,
    pub fg1: Color,
    pub grey0: Color,
    pub grey1: Color,
    pub grey2: Color,

    // Palette colors
    pub red: Color,
    pub orange: Color,
    pub yellow: Color,
    pub green: Color,
    pub aqua: Color,
    pub blue: Color,
    pub purple: Color,

    // Semantic colors
    pub selection: Color,
    pub unread: Color,
    pub read: Color,
    pub error: Color,
    pub success: Color,
    pub warning: Color,
    pub info: Color,
    pub accent: Color,
}

impl Default for Theme {
    fn default() -> Self {
        // Default to Gruvbox Dark
        Self {
            bg0: Color::Rgb(0x28, 0x28, 0x28),
            bg1: Color::Rgb(0x32, 0x30, 0x2f),
            bg2: Color::Rgb(0x45, 0x40, 0x3d),
            bg3: Color::Rgb(0x50, 0x49, 0x45),
            fg0: Color::Rgb(0xd4, 0xbe, 0x98),
            fg1: Color::Rgb(0xdd, 0xc7, 0xa1),
            grey0: Color::Rgb(0x7c, 0x6f, 0x64),
            grey1: Color::Rgb(0x92, 0x83, 0x74),
            grey2: Color::Rgb(0xa8, 0x99, 0x84),
            red: Color::Rgb(0xea, 0x69, 0x62),
            orange: Color::Rgb(0xe7, 0x8a, 0x4e),
            yellow: Color::Rgb(0xd8, 0xa6, 0x57),
            green: Color::Rgb(0xa9, 0xb6, 0x65),
            aqua: Color::Rgb(0x89, 0xb4, 0x82),
            blue: Color::Rgb(0x7d, 0xae, 0xa3),
            purple: Color::Rgb(0xd3, 0x86, 0x9b),
            selection: Color::Rgb(0x45, 0x40, 0x3d),
            unread: Color::Rgb(0xd8, 0xa6, 0x57),
            read: Color::Rgb(0x92, 0x83, 0x74),
            error: Color::Rgb(0xea, 0x69, 0x62),
            success: Color::Rgb(0xa9, 0xb6, 0x65),
            warning: Color::Rgb(0xe7, 0x8a, 0x4e),
            info: Color::Rgb(0x7d, 0xae, 0xa3),
            accent: Color::Rgb(0x89, 0xb4, 0x82),
        }
    }
}

