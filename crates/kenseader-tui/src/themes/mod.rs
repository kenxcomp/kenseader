//! Theme registry and loader
//!
//! Provides 24 built-in themes with user customization support.

mod catppuccin;
mod dracula;
mod everforest;
mod gruvbox;
mod kanagawa;
mod monokai;
mod nord;
mod one_dark;
mod rose_pine;
mod solarized;
mod tokyo_night;
mod zenburn;

use kenseader_core::config::{ThemeColorOverrides, ThemeConfig};
use ratatui::style::Color;

use crate::theme::Theme;

/// Parse a hex color string into a ratatui Color
/// Accepts formats: "#RRGGBB", "RRGGBB", "#RGB", "RGB"
pub fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim().trim_start_matches('#');

    match hex.len() {
        // Short form: RGB -> RRGGBB
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            Some(Color::Rgb(r, g, b))
        }
        // Full form: RRGGBB
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        _ => None,
    }
}

/// Load a theme by name from config
pub fn load_theme(config: &ThemeConfig) -> Theme {
    let base = match config.name.to_lowercase().as_str() {
        // Catppuccin variants
        "catppuccin-latte" => catppuccin::latte(),
        "catppuccin-frappe" => catppuccin::frappe(),
        "catppuccin-macchiato" => catppuccin::macchiato(),
        "catppuccin-mocha" => catppuccin::mocha(),

        // Gruvbox variants
        "gruvbox-light" => gruvbox::light(),
        "gruvbox-dark" => gruvbox::dark(),

        // Dracula
        "dracula" => dracula::default(),

        // Nord
        "nord" => nord::default(),

        // Tokyo Night variants
        "tokyo-night" | "tokyo-night-night" => tokyo_night::night(),
        "tokyo-night-storm" => tokyo_night::storm(),
        "tokyo-night-day" => tokyo_night::day(),

        // One Dark
        "one-dark" | "onedark" => one_dark::default(),

        // Solarized variants
        "solarized-light" => solarized::light(),
        "solarized-dark" => solarized::dark(),

        // Everforest variants
        "everforest-light" => everforest::light(),
        "everforest-dark" => everforest::dark(),

        // Kanagawa variants
        "kanagawa" | "kanagawa-wave" => kanagawa::wave(),
        "kanagawa-dragon" => kanagawa::dragon(),
        "kanagawa-lotus" => kanagawa::lotus(),

        // Rose Pine variants
        "rose-pine" | "rose-pine-main" => rose_pine::main(),
        "rose-pine-moon" => rose_pine::moon(),
        "rose-pine-dawn" => rose_pine::dawn(),

        // Monokai
        "monokai" => monokai::default(),

        // Zenburn
        "zenburn" => zenburn::default(),

        // Default fallback
        _ => gruvbox::dark(),
    };

    apply_overrides(base, &config.colors)
}

/// Apply user color overrides to a base theme
fn apply_overrides(mut theme: Theme, overrides: &ThemeColorOverrides) -> Theme {
    if let Some(ref hex) = overrides.bg0 {
        if let Some(color) = parse_hex_color(hex) {
            theme.bg0 = color;
        }
    }
    if let Some(ref hex) = overrides.bg1 {
        if let Some(color) = parse_hex_color(hex) {
            theme.bg1 = color;
        }
    }
    if let Some(ref hex) = overrides.bg2 {
        if let Some(color) = parse_hex_color(hex) {
            theme.bg2 = color;
        }
    }
    if let Some(ref hex) = overrides.fg0 {
        if let Some(color) = parse_hex_color(hex) {
            theme.fg0 = color;
        }
    }
    if let Some(ref hex) = overrides.fg1 {
        if let Some(color) = parse_hex_color(hex) {
            theme.fg1 = color;
        }
    }
    if let Some(ref hex) = overrides.accent {
        if let Some(color) = parse_hex_color(hex) {
            theme.accent = color;
        }
    }
    if let Some(ref hex) = overrides.selection {
        if let Some(color) = parse_hex_color(hex) {
            theme.selection = color;
        }
    }
    if let Some(ref hex) = overrides.unread {
        if let Some(color) = parse_hex_color(hex) {
            theme.unread = color;
        }
    }
    if let Some(ref hex) = overrides.read {
        if let Some(color) = parse_hex_color(hex) {
            theme.read = color;
        }
    }
    if let Some(ref hex) = overrides.error {
        if let Some(color) = parse_hex_color(hex) {
            theme.error = color;
        }
    }
    if let Some(ref hex) = overrides.success {
        if let Some(color) = parse_hex_color(hex) {
            theme.success = color;
        }
    }
    if let Some(ref hex) = overrides.warning {
        if let Some(color) = parse_hex_color(hex) {
            theme.warning = color;
        }
    }
    if let Some(ref hex) = overrides.info {
        if let Some(color) = parse_hex_color(hex) {
            theme.info = color;
        }
    }

    theme
}

/// Get list of available theme names
pub fn available_themes() -> Vec<&'static str> {
    vec![
        "catppuccin-latte",
        "catppuccin-frappe",
        "catppuccin-macchiato",
        "catppuccin-mocha",
        "gruvbox-light",
        "gruvbox-dark",
        "dracula",
        "nord",
        "tokyo-night",
        "tokyo-night-storm",
        "tokyo-night-day",
        "one-dark",
        "solarized-light",
        "solarized-dark",
        "everforest-light",
        "everforest-dark",
        "kanagawa-wave",
        "kanagawa-dragon",
        "kanagawa-lotus",
        "rose-pine",
        "rose-pine-moon",
        "rose-pine-dawn",
        "monokai",
        "zenburn",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color_6digit() {
        let color = parse_hex_color("#ff5500").unwrap();
        assert!(matches!(color, Color::Rgb(255, 85, 0)));
    }

    #[test]
    fn test_parse_hex_color_3digit() {
        let color = parse_hex_color("#f50").unwrap();
        assert!(matches!(color, Color::Rgb(255, 85, 0)));
    }

    #[test]
    fn test_parse_hex_color_no_hash() {
        let color = parse_hex_color("ff5500").unwrap();
        assert!(matches!(color, Color::Rgb(255, 85, 0)));
    }

    #[test]
    fn test_parse_hex_color_invalid() {
        assert!(parse_hex_color("invalid").is_none());
        assert!(parse_hex_color("#gg0000").is_none());
    }

    #[test]
    fn test_load_theme_default() {
        let config = ThemeConfig::default();
        let theme = load_theme(&config);
        // Should load gruvbox-dark
        assert!(matches!(theme.bg0, Color::Rgb(0x28, 0x28, 0x28)));
    }

    #[test]
    fn test_load_theme_with_override() {
        let config = ThemeConfig {
            name: "gruvbox-dark".to_string(),
            colors: ThemeColorOverrides {
                unread: Some("#ff0000".to_string()),
                ..Default::default()
            },
        };
        let theme = load_theme(&config);
        assert!(matches!(theme.unread, Color::Rgb(255, 0, 0)));
    }
}
