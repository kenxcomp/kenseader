use ratatui::style::Color;

/// Gruvbox Material Dark Medium palette
pub struct GruvboxMaterial;

impl GruvboxMaterial {
    // Background colors
    pub const BG_DIM: Color = Color::Rgb(0x1b, 0x1b, 0x1b);      // #1b1b1b
    pub const BG0: Color = Color::Rgb(0x28, 0x28, 0x28);         // #282828
    pub const BG1: Color = Color::Rgb(0x32, 0x30, 0x2f);         // #32302f
    pub const BG2: Color = Color::Rgb(0x45, 0x40, 0x3d);         // #45403d
    pub const BG3: Color = Color::Rgb(0x50, 0x49, 0x45);         // #504945

    // Foreground colors
    pub const FG0: Color = Color::Rgb(0xd4, 0xbe, 0x98);         // #d4be98
    pub const FG1: Color = Color::Rgb(0xdd, 0xc7, 0xa1);         // #ddc7a1
    pub const GREY0: Color = Color::Rgb(0x7c, 0x6f, 0x64);       // #7c6f64
    pub const GREY1: Color = Color::Rgb(0x92, 0x83, 0x74);       // #928374
    pub const GREY2: Color = Color::Rgb(0xa8, 0x99, 0x84);       // #a89984

    // Accent colors
    pub const RED: Color = Color::Rgb(0xea, 0x69, 0x62);         // #ea6962
    pub const ORANGE: Color = Color::Rgb(0xe7, 0x8a, 0x4e);      // #e78a4e
    pub const YELLOW: Color = Color::Rgb(0xd8, 0xa6, 0x57);      // #d8a657
    pub const GREEN: Color = Color::Rgb(0xa9, 0xb6, 0x65);       // #a9b665
    pub const AQUA: Color = Color::Rgb(0x89, 0xb4, 0x82);        // #89b482
    pub const BLUE: Color = Color::Rgb(0x7d, 0xae, 0xa3);        // #7daea3
    pub const PURPLE: Color = Color::Rgb(0xd3, 0x86, 0x9b);      // #d3869b

    // Semantic colors
    pub const SELECTION: Color = Self::BG2;
    pub const CURSOR: Color = Self::FG0;
    pub const UNREAD: Color = Self::YELLOW;
    pub const READ: Color = Self::GREY1;
    pub const ERROR: Color = Self::RED;
    pub const SUCCESS: Color = Self::GREEN;
    pub const WARNING: Color = Self::ORANGE;
    pub const INFO: Color = Self::BLUE;
    pub const ACCENT: Color = Self::AQUA;
}
