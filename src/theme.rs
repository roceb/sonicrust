use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug,Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreset {
    #[default]
    Default,
    Everforest,
    Nord,
    Dracula,
    Catppuccin,
    Custom,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Theme {
    #[serde(default)]
    pub preset: ThemePreset,
    // Overide colors when preset = custom
    pub bg: Option<String>,
    pub fg: Option<String>,
    pub accent: Option<String>,
    pub highlight_bg: Option<String>,
    pub highlight_fg: Option<String>,
    pub border_active: Option<String>,
    pub border_inactive: Option<String>,
    pub playing_color: Option<String>,
    pub artist_color: Option<String>,
    pub album_color: Option<String>,
    pub muted_color: Option<String>,
    pub bold: bool,
}

#[derive(Clone, Debug)]
pub struct ResolvedTheme {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub highlight_bg: Color,
    pub highlight_fg: Color,
    pub border_active: Color,
    pub border_inactive: Color,
    pub playing_color: Color,
    pub artist_color: Color,
    pub album_color: Color,
    pub muted_color: Color,
    pub bold: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            preset: ThemePreset::Default,
            bg: None,
            fg: None,
            accent: None,
            highlight_bg: None,
            highlight_fg: None,
            border_active: None,
            border_inactive: None,
            playing_color: None,
            artist_color: None,
            album_color: None,
            muted_color: None,
            bold: true,
        }
    }
}

impl Theme {
    pub fn resolve(&self) -> ResolvedTheme {
        let preset = self.preset_colors();

        ResolvedTheme {
            bg: self.parse_color_or(&self.bg, preset.bg),
            fg: self.parse_color_or(&self.fg, preset.fg),
            accent: self.parse_color_or(&self.accent, preset.accent),
            highlight_bg: self.parse_color_or(&self.highlight_bg, preset.highlight_bg),
            highlight_fg: self.parse_color_or(&self.highlight_fg, preset.highlight_fg),
            border_active: self.parse_color_or(&self.border_active, preset.border_active),
            border_inactive: self.parse_color_or(&self.border_inactive, preset.border_inactive),
            playing_color: self.parse_color_or(&self.playing_color, preset.playing_color),
            artist_color: self.parse_color_or(&self.artist_color, preset.artist_color),
            album_color: self.parse_color_or(&self.album_color, preset.album_color),
            muted_color: self.parse_color_or(&self.muted_color, preset.muted_color),
            bold: self.bold,
        }
    }

    fn parse_color_or(&self, override_color: &Option<String>, fallback: Color) -> Color {
        override_color
            .as_deref()
            .and_then(|s| Self::parse_color(s))
            .unwrap_or(fallback)
    }

    fn parse_color(s: &str) -> Option<Color> {
        // Handle hex colors like #a7c080
        if let Some(hex) = s.strip_prefix('#') &&
            hex.len() == 6 {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                return Some(Color::Rgb(r, g, b));
        }
        // Handle named colors
        match s.to_lowercase().as_str() {
            "black" => Some(Color::Black),
            "red" => Some(Color::Red),
            "green" => Some(Color::Green),
            "yellow" => Some(Color::Yellow),
            "blue" => Some(Color::Blue),
            "magenta" => Some(Color::Magenta),
            "cyan" => Some(Color::Cyan),
            "white" => Some(Color::White),
            "darkgray" | "darkgrey" => Some(Color::DarkGray),
            "lightred" => Some(Color::LightRed),
            "lightgreen" => Some(Color::LightGreen),
            "lightyellow" => Some(Color::LightYellow),
            "lightblue" => Some(Color::LightBlue),
            "lightmagenta" => Some(Color::LightMagenta),
            "lightcyan" => Some(Color::LightCyan),
            "gray" | "grey" => Some(Color::Gray),
            "reset" | "" => Some(Color::Reset),
            _ => None,
        }
    }

    fn preset_colors(&self) -> ResolvedTheme {
        match self.preset {
            ThemePreset::Everforest => ResolvedTheme {
                bg: Color::Rgb(45, 53, 59),
                fg: Color::Rgb(211, 198, 170),
                accent: Color::Rgb(167, 192, 128),      // green
                highlight_bg: Color::Rgb(82, 97, 89),
                highlight_fg: Color::Rgb(211, 198, 170),
                border_active: Color::Rgb(167, 192, 128),
                border_inactive: Color::Rgb(82, 97, 89),
                playing_color: Color::Rgb(167, 192, 128),
                artist_color: Color::Rgb(214, 153, 104), // orange
                album_color: Color::Rgb(131, 192, 146),  // aqua
                muted_color: Color::Rgb(131, 145, 141),  // gray
                bold: true,
            },
            ThemePreset::Nord => ResolvedTheme {
                bg: Color::Rgb(46, 52, 64),             // nord0
                fg: Color::Rgb(236, 239, 244),          // nord6
                accent: Color::Rgb(136, 192, 208),      // nord8
                highlight_bg: Color::Rgb(67, 76, 94),   // nord2
                highlight_fg: Color::Rgb(236, 239, 244),
                border_active: Color::Rgb(136, 192, 208),
                border_inactive: Color::Rgb(76, 86, 106), // nord3
                playing_color: Color::Rgb(163, 190, 140), // nord14 green
                artist_color: Color::Rgb(235, 203, 139),  // nord13 yellow
                album_color: Color::Rgb(136, 192, 208),   // nord8 frost
                muted_color: Color::Rgb(76, 86, 106),
                bold: true,
            },
            ThemePreset::Dracula => ResolvedTheme {
                bg: Color::Rgb(40, 42, 54),
                fg: Color::Rgb(248, 248, 242),
                accent: Color::Rgb(189, 147, 249),      // purple
                highlight_bg: Color::Rgb(68, 71, 90),
                highlight_fg: Color::Rgb(248, 248, 242),
                border_active: Color::Rgb(189, 147, 249),
                border_inactive: Color::Rgb(68, 71, 90),
                playing_color: Color::Rgb(80, 250, 123),  // green
                artist_color: Color::Rgb(255, 184, 108),  // orange
                album_color: Color::Rgb(139, 233, 253),   // cyan
                muted_color: Color::Rgb(98, 114, 164),
                bold: true,
            },
            ThemePreset::Catppuccin => ResolvedTheme {
                // Catppuccin Mocha
                bg: Color::Rgb(30, 30, 46),
                fg: Color::Rgb(205, 214, 244),
                accent: Color::Rgb(137, 180, 250),      // blue
                highlight_bg: Color::Rgb(49, 50, 68),
                highlight_fg: Color::Rgb(205, 214, 244),
                border_active: Color::Rgb(137, 180, 250),
                border_inactive: Color::Rgb(88, 91, 112),
                playing_color: Color::Rgb(166, 227, 161), // green
                artist_color: Color::Rgb(250, 179, 135),  // peach
                album_color: Color::Rgb(137, 220, 235),   // teal
                muted_color: Color::Rgb(108, 112, 134),
                bold: true,
            },
            // Default and Custom fall through to default colors
            ThemePreset::Default | ThemePreset::Custom => ResolvedTheme {
                bg: Color::Reset,
                fg: Color::White,
                accent: Color::Cyan,
                highlight_bg: Color::DarkGray,
                highlight_fg: Color::White,
                border_active: Color::Cyan,
                border_inactive: Color::DarkGray,
                playing_color: Color::LightGreen,
                artist_color: Color::Yellow,
                album_color: Color::Cyan,
                muted_color: Color::DarkGray,
                bold: true,
            },
        }
    }
}
