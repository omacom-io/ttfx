//! Colored bar that sweeps across the canvas and reveals the input text.

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Sweeps a bright bar across the text, revealing characters as it passes.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sweep;

impl Sweep {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Sweep {
    fn name(&self) -> &str {
        "sweep"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_input(input, TerminalConfig::default());
        let mut frames = Vec::new();

        if terminal.character_count() == 0 {
            frames.push(terminal.render_frame());
            return frames;
        }

        terminal.hide_all();
        frames.push(terminal.render_frame());

        let (min_col, max_col) = {
            let mut min_col = i32::MAX;
            let mut max_col = i32::MIN;
            for ch in terminal.get_characters() {
                min_col = min_col.min(ch.input_coord.column);
                max_col = max_col.max(ch.input_coord.column);
            }
            (min_col, max_col)
        };

        let bar_width = 7_i32;
        let trail = Gradient::new(
            &[
                Color::rgb(0xff, 0xff, 0xff),
                Color::rgb(0x9e, 0xef, 0xff),
                Color::rgb(0x3d, 0xb8, 0xff),
                Color::rgb(0x1f, 0x6f, 0xd1),
            ],
            5,
        );

        for front in min_col..=max_col.saturating_add(bar_width) {
            for ch in terminal.get_characters_mut() {
                let dist = front - ch.input_coord.column;
                if dist < 0 {
                    ch.is_visible = false;
                    continue;
                }
                ch.is_visible = true;
                let symbol = ch.input_symbol.clone();
                let input_fg = ch.input_fg;
                let input_bg = ch.input_bg;
                if dist < bar_width {
                    let denom = f64::from((bar_width - 1).max(1));
                    let color = trail
                        .mapped_color(f64::from(dist) / denom)
                        .unwrap_or(Color::rgb(0xff, 0xff, 0xff));
                    ch.animation
                        .set_appearance(&symbol, Some(ColorPair::new(Some(color), input_bg)));
                } else if input_fg.is_some() || input_bg.is_some() {
                    ch.animation
                        .set_appearance(&symbol, Some(ColorPair::new(input_fg, input_bg)));
                } else {
                    ch.animation.set_appearance(&symbol, None);
                }
            }
            frames.push(terminal.render_frame());
        }

        for _ in 0..10 {
            frames.push(terminal.render_frame());
        }

        frames
    }
}
