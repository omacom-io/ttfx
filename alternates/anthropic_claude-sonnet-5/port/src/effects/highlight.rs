//! Highlight: a bright band sweeps diagonally across the text, briefly
//! lightening each character as the band passes over it before it settles
//! back to a dimmer base tone (mirrors
//! terminaltexteffects/effects/effect_highlight.py's diagonal sweep-and-fade
//! shine, expressed here as precomputed frame strings since this simplified
//! engine skeleton has no path/scene-driven effect runtime yet).

use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, ColorPair, Gradient};

pub struct Highlight {
    pub highlight_color: Color,
    pub base_color: Color,
    pub gradient_steps: usize,
}

impl Highlight {
    pub fn new() -> Self {
        Highlight {
            highlight_color: Color::Rgb(255, 255, 255),
            base_color: Color::Rgb(120, 120, 120),
            gradient_steps: 8,
        }
    }
}

impl Effect for Highlight {
    fn name(&self) -> &str {
        "highlight"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let terminal = Terminal::new(input);
        let width = terminal.config.width;
        let height = terminal.config.height;

        // Symmetric gradient: base -> highlight -> base, so a character
        // brightens as the sweep line approaches and dims again as it
        // passes, matching the shine-then-fade look of the Python original.
        let gradient = Gradient::new(
            &[self.base_color, self.highlight_color, self.base_color],
            self.gradient_steps.max(1),
        );
        let half_len = (gradient.len() / 2) as isize;

        // Diagonal sweep over column+row, moving the band from the
        // top-left corner to the bottom-right corner of the canvas, with
        // enough trailing frames for the gradient's falloff to fully clear
        // the last diagonal.
        let max_diag = if width + height >= 2 { (width + height - 2) as isize } else { 0 };
        let total_frames = (max_diag + half_len + 1).max(1) as usize;

        let mut frames = Vec::with_capacity(total_frames);

        for t in 0..total_frames {
            let mut rows: Vec<String> = Vec::with_capacity(height);
            for row in 0..height {
                let mut line = String::new();
                for column in 0..width {
                    let symbol = terminal.canvas.get(column, row).unwrap_or(' ');
                    let diag = (column + row) as isize;
                    let offset = diag - t as isize;
                    let dist = offset.unsigned_abs() as isize;

                    let mut visual = CharacterVisual::new(symbol);
                    if dist <= half_len {
                        let gradient_index = (half_len + offset).clamp(0, gradient.len() as isize - 1) as usize;
                        if let Some(color) = gradient.get(gradient_index) {
                            visual.colors = Some(ColorPair::new(Some(color), None));
                            visual.formatted_symbol = visual.format_symbol();
                        }
                    }
                    line.push_str(&visual.formatted_symbol);
                }
                rows.push(line);
            }
            frames.push(rows.join("\n"));
        }

        frames
    }
}
