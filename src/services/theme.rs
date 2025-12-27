//! Theme service for loading wallust colors.

use anyhow::Result;
use ratatui::style::Color;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

/// Wallust color palette loaded from JSON.
#[derive(Debug, Clone, Deserialize)]
pub struct WallustColors {
    pub background: String,
    pub foreground: String,
    pub cursor: String,
    pub color0: String,
    pub color1: String,
    pub color2: String,
    pub color3: String,
    pub color4: String,
    pub color5: String,
    pub color6: String,
    pub color7: String,
    pub color8: String,
    pub color9: String,
    pub color10: String,
    pub color11: String,
    pub color12: String,
    pub color13: String,
    pub color14: String,
    pub color15: String,
}

/// Theme with ratatui colors.
#[derive(Debug, Clone)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub cursor: Color,
    pub color0: Color,
    pub color1: Color,
    pub color2: Color,
    pub color3: Color,
    pub color4: Color,
    pub color5: Color,
    pub color6: Color,
    pub color7: Color,
    pub color8: Color,
    pub color9: Color,
    pub color10: Color,
    pub color11: Color,
    pub color12: Color,
    pub color13: Color,
    pub color14: Color,
    pub color15: Color,
}

impl Theme {
    /// Load theme from wallust colors file, falling back to defaults.
    pub fn load() -> Self {
        let path = Self::colors_path();
        if path.exists() {
            Self::from_file(&path).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Get the path to wallust colors file.
    fn colors_path() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("wallust")
            .join("colors-original.json")
    }

    /// Load theme from a specific file.
    fn from_file(path: &PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let colors: WallustColors = serde_json::from_str(&content)?;
        Ok(Self::from_wallust(colors))
    }

    /// Convert wallust colors to theme.
    fn from_wallust(colors: WallustColors) -> Self {
        Self {
            background: parse_hex(&colors.background),
            foreground: parse_hex(&colors.foreground),
            cursor: parse_hex(&colors.cursor),
            color0: parse_hex(&colors.color0),
            color1: parse_hex(&colors.color1),
            color2: parse_hex(&colors.color2),
            color3: parse_hex(&colors.color3),
            color4: parse_hex(&colors.color4),
            color5: parse_hex(&colors.color5),
            color6: parse_hex(&colors.color6),
            color7: parse_hex(&colors.color7),
            color8: parse_hex(&colors.color8),
            color9: parse_hex(&colors.color9),
            color10: parse_hex(&colors.color10),
            color11: parse_hex(&colors.color11),
            color12: parse_hex(&colors.color12),
            color13: parse_hex(&colors.color13),
            color14: parse_hex(&colors.color14),
            color15: parse_hex(&colors.color15),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: Color::Black,
            foreground: Color::White,
            cursor: Color::White,
            color0: Color::Black,
            color1: Color::Red,
            color2: Color::Green,
            color3: Color::Yellow,
            color4: Color::Blue,
            color5: Color::Magenta,
            color6: Color::Cyan,
            color7: Color::Gray,
            color8: Color::DarkGray,
            color9: Color::LightRed,
            color10: Color::LightGreen,
            color11: Color::LightYellow,
            color12: Color::LightBlue,
            color13: Color::LightMagenta,
            color14: Color::LightCyan,
            color15: Color::White,
        }
    }
}

/// Parse a hex color string like "#RRGGBB" to a ratatui Color.
fn parse_hex(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Color::White;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);

    Color::Rgb(r, g, b)
}
