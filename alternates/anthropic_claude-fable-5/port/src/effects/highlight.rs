//! Highlight effect: a specular highlight sweeps diagonally across the text.
//!
//! Port of terminaltexteffects/effects/effect_highlight.py. Characters are
//! displayed in their final gradient colors, then a band of brightened color
//! travels across the canvas from the bottom-left to the top-right, one
//! diagonal at a time, before settling back to the final colors.

use std::collections::BTreeMap;

use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair, Gradient};

pub struct Highlight {
    /// Percent brightness increase applied at the peak of the highlight band.
    pub highlight_brightness_increase: u32,
    /// Width of the highlight band (frames of ramp-up/ramp-down per character).
    pub highlight_width: usize,
    /// Stops of the final (resting) gradient across the canvas.
    pub final_gradient_stops: Vec<Color>,
    /// Interpolation steps between gradient stops.
    pub final_gradient_steps: usize,
}

impl Highlight {
    pub fn new() -> Self {
        Highlight {
            highlight_brightness_increase: 75,
            highlight_width: 8,
            final_gradient_stops: vec![
                Color::from_hex("8A008A").expect("valid hex"),
                Color::from_hex("00D1FF").expect("valid hex"),
                Color::from_hex("FFFFFF").expect("valid hex"),
            ],
            final_gradient_steps: 12,
        }
    }

    /// Brighten a color by moving each channel toward white by `pct` percent.
    fn brighten(color: Color, pct: u32) -> Color {
        let factor = pct as f64 / 100.0;
        let adjust = |c: u8| -> u8 {
            (c as f64 + (255.0 - c as f64) * factor)
                .round()
                .clamp(0.0, 255.0) as u8
        };
        Color::new(adjust(color.r), adjust(color.g), adjust(color.b))
    }
}

impl Default for Highlight {
    fn default() -> Self {
        Highlight::new()
    }
}

impl Effect for Highlight {
    fn name(&self) -> &str {
        "highlight"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let height = terminal.canvas.height as i32;
        let final_gradient = Gradient::new(&self.final_gradient_stops, self.final_gradient_steps);

        // Group character ids by diagonal (column + row): ascending keys sweep
        // the band from the bottom-left corner to the top-right corner.
        let mut groups: BTreeMap<i32, Vec<usize>> = BTreeMap::new();

        for character in terminal.get_characters_mut() {
            let coord = character.input_coord;
            // Vertical gradient mapping: bottom row -> first stop, top -> last.
            let fraction = if height > 1 {
                (coord.row - 1) as f64 / (height - 1) as f64
            } else {
                0.0
            };
            let base = final_gradient
                .get_color_at_fraction(fraction)
                .unwrap_or_else(|| Color::new(255, 255, 255));
            let bright = Self::brighten(base, self.highlight_brightness_increase);

            // Highlight scene: base -> brightened -> base, one tick per shade.
            let sweep = Gradient::new(&[base, bright, base], self.highlight_width.max(1));
            let scene = character.animation.new_scene("highlight", false);
            for color in &sweep.spectrum {
                scene.add_frame(character.input_symbol, 1, ColorPair::fg(*color), false);
            }

            // Rest in the final gradient color until (and after) the band hits.
            character.animation.current_visual =
                CharacterVisual::new(character.input_symbol, false, ColorPair::fg(base));
            character.is_visible = true;

            groups
                .entry(coord.column + coord.row)
                .or_default()
                .push(character.character_id);
        }

        let mut frames = Vec::new();
        frames.push(terminal.get_formatted_output_string());

        let group_queue: Vec<Vec<usize>> = groups.into_values().collect();
        let mut next_group = 0usize;

        loop {
            // Launch the next diagonal's highlight scenes, one group per tick.
            if next_group < group_queue.len() {
                let ids = &group_queue[next_group];
                for character in terminal.get_characters_mut() {
                    if ids.contains(&character.character_id) {
                        character.animation.activate_scene("highlight");
                    }
                }
                next_group += 1;
            }

            let active = terminal.tick();
            frames.push(terminal.get_formatted_output_string());

            if next_group >= group_queue.len() && active == 0 {
                break;
            }
        }

        frames
    }
}
