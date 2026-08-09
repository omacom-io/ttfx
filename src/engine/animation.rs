//! CharacterVisual and the Animation state, ported from engine/animation.py.
//! M0 ships the visual + input-color handling; Scene machinery lands in M1.

use std::collections::HashMap;

use crate::utils::ansi::{self, ColorCode};
use crate::utils::graphics::{Color, ColorPair};
use crate::utils::hexterm;

/// Handling of preexisting SGR colors in the input (TerminalConfig option).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExistingColorHandling {
    Always,
    Dynamic,
    Ignore,
}

/// animation.CharacterVisual: symbol + modes + resolved color codes, with the
/// fully formatted ANSI string precomputed at construction (upstream __post_init__).
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterVisual {
    pub symbol: String,
    pub bold: bool,
    pub dim: bool, // stored but never emitted, faithfully
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub reverse: bool,
    pub hidden: bool,
    pub strike: bool,
    pub colors: Option<ColorPair>,
    pub fg_color_code: Option<ColorCode>,
    pub bg_color_code: Option<ColorCode>,
    pub formatted_symbol: String,
}

#[derive(Debug, Clone, Default)]
pub struct VisualParams {
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub reverse: bool,
    pub hidden: bool,
    pub strike: bool,
    pub colors: Option<ColorPair>,
    pub fg_color_code: Option<ColorCode>,
    pub bg_color_code: Option<ColorCode>,
}

impl CharacterVisual {
    pub fn new(symbol: &str, p: VisualParams) -> Self {
        let mut vis = CharacterVisual {
            symbol: symbol.to_string(),
            bold: p.bold,
            dim: p.dim,
            italic: p.italic,
            underline: p.underline,
            blink: p.blink,
            reverse: p.reverse,
            hidden: p.hidden,
            strike: p.strike,
            colors: p.colors,
            fg_color_code: p.fg_color_code,
            bg_color_code: p.bg_color_code,
            formatted_symbol: String::new(),
        };
        vis.formatted_symbol = vis.format_symbol();
        vis
    }

    pub fn plain(symbol: &str) -> Self {
        CharacterVisual::new(symbol, VisualParams::default())
    }

    /// SGR emission in upstream's fixed order: bold, italic, underline, blink,
    /// reverse, hidden, strike, fg, bg; `dim` intentionally omitted; bare symbol
    /// when nothing applies.
    fn format_symbol(&self) -> String {
        let mut fmt = String::new();
        if self.bold {
            fmt.push_str(ansi::BOLD);
        }
        if self.italic {
            fmt.push_str(ansi::ITALIC);
        }
        if self.underline {
            fmt.push_str(ansi::UNDERLINE);
        }
        if self.blink {
            fmt.push_str(ansi::BLINK);
        }
        if self.reverse {
            fmt.push_str(ansi::REVERSE);
        }
        if self.hidden {
            fmt.push_str(ansi::HIDDEN);
        }
        if self.strike {
            fmt.push_str(ansi::STRIKETHROUGH);
        }
        if let Some(code) = &self.fg_color_code {
            ansi::fg(code, &mut fmt);
        }
        if let Some(code) = &self.bg_color_code {
            ansi::bg(code, &mut fmt);
        }
        if fmt.is_empty() {
            self.symbol.clone()
        } else {
            format!("{fmt}{}{}", self.symbol, ansi::RESET_ALL)
        }
    }
}

/// The Animation state attached to every character (engine/animation.py Animation).
/// Scene storage/stepping arrives in M1; the fields below are what input parsing
/// and rendering need.
#[derive(Debug, Clone)]
pub struct Animation {
    pub use_xterm_colors: bool,
    pub no_color: bool,
    pub existing_color_handling: ExistingColorHandling,
    pub input_fg_color: Option<Color>,
    pub input_bg_color: Option<Color>,
    pub input_bold: bool,
    /// Per-animation memo of rgb hex -> xterm code (upstream instance-level map).
    pub xterm_color_map: HashMap<String, u8>,
    pub current_character_visual: CharacterVisual,
}

impl Animation {
    pub fn new(input_symbol: &str) -> Self {
        Animation {
            use_xterm_colors: false,
            no_color: false,
            existing_color_handling: ExistingColorHandling::Ignore,
            input_fg_color: None,
            input_bg_color: None,
            input_bold: false,
            xterm_color_map: HashMap::new(),
            current_character_visual: CharacterVisual::plain(input_symbol),
        }
    }

    /// Animation._get_color_code: no_color -> None; xterm mode -> code (memoized
    /// nearest-match for RGB); else the hex string.
    pub fn get_color_code(&mut self, color: Option<&Color>) -> Option<ColorCode> {
        let color = color?;
        if self.no_color {
            return None;
        }
        if self.use_xterm_colors {
            if let Some(code) = color.xterm_color {
                return Some(ColorCode::Xterm(code));
            }
            if let Some(&code) = self.xterm_color_map.get(&color.rgb_color) {
                return Some(ColorCode::Xterm(code));
            }
            let code = hexterm::hex_to_xterm(&color.rgb_color);
            self.xterm_color_map.insert(color.rgb_color.clone(), code);
            return Some(ColorCode::Xterm(code));
        }
        Some(ColorCode::Rgb(color.rgb_color.clone()))
    }

    /// Animation.set_appearance. `uses_input_preexisting_colors` comes from the
    /// owning character (passed in because the Animation is stored inside it).
    pub fn set_appearance(
        &mut self,
        input_symbol: &str,
        uses_input_preexisting_colors: bool,
        symbol: Option<&str>,
        colors: Option<ColorPair>,
    ) {
        let symbol = symbol.unwrap_or(input_symbol);
        let mut colors = colors.unwrap_or_default();
        let mut bold = false;
        if self.existing_color_handling == ExistingColorHandling::Always && uses_input_preexisting_colors {
            colors = ColorPair::new(self.input_fg_color.clone(), self.input_bg_color.clone());
            bold = self.input_bold;
        }
        let fg_code = self.get_color_code(colors.fg_color.as_ref());
        let bg_code = self.get_color_code(colors.bg_color.as_ref());
        self.current_character_visual = CharacterVisual::new(
            symbol,
            VisualParams {
                bold,
                colors: Some(colors),
                fg_color_code: fg_code,
                bg_color_code: bg_code,
                ..Default::default()
            },
        );
    }
}
