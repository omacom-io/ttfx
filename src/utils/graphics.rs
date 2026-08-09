//! Color and ColorPair, ported from utils/graphics.py. Gradient lands in M1.

use crate::utils::hexterm;

/// The original constructor argument, preserved because upstream `Color.__eq__`
/// and `__hash__` compare `color_arg` — `Color(255) != Color("ffffff")` even
/// when they resolve to the same RGB. Dict/set keying depends on this.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ColorArg {
    Xterm(u8),
    Hex(String), // stored stripped of '#', case preserved (upstream strips '#' only)
}

#[derive(Debug, Clone)]
pub struct Color {
    pub color_arg: ColorArg,
    /// Some(code) when constructed from an xterm int, None for hex strings.
    pub xterm_color: Option<u8>,
    /// hex string without '#'
    pub rgb_color: String,
}

impl PartialEq for Color {
    fn eq(&self, other: &Self) -> bool {
        self.color_arg == other.color_arg
    }
}
impl Eq for Color {}
impl std::hash::Hash for Color {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.color_arg.hash(state);
    }
}

impl Color {
    pub fn from_xterm(code: u8) -> Self {
        Color {
            color_arg: ColorArg::Xterm(code),
            xterm_color: Some(code),
            rgb_color: hexterm::xterm_to_hex(code).to_string(),
        }
    }

    /// Hex-string constructor. Errors mirror upstream ValueError.
    pub fn from_hex(hex: &str) -> Result<Self, String> {
        let stripped = hex.trim_matches('#');
        if !hexterm::is_valid_hex_color(stripped) {
            return Err(
                "Invalid color value. Color must be an XTerm-256 color code or an RGB hex color string. \
                 Example: 255 or 'ffffff' or '#ffffff'"
                    .to_string(),
            );
        }
        Ok(Color {
            color_arg: ColorArg::Hex(stripped.to_string()),
            xterm_color: None,
            rgb_color: stripped.to_string(),
        })
    }

    pub fn rgb_ints(&self) -> (u8, u8, u8) {
        let s = &self.rgb_color;
        (
            u8::from_str_radix(&s[0..2], 16).unwrap(),
            u8::from_str_radix(&s[2..4], 16).unwrap(),
            u8::from_str_radix(&s[4..6], 16).unwrap(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColorPair {
    pub fg_color: Option<Color>,
    pub bg_color: Option<Color>,
}

impl ColorPair {
    pub fn new(fg: Option<Color>, bg: Option<Color>) -> Self {
        ColorPair { fg_color: fg, bg_color: bg }
    }
}
