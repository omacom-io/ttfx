//! Slice effect: the text is sliced horizontally at its vertical center; the
//! left half of each row slides in from above the canvas while the right half
//! of the opposite row slides in from below, meeting at the input coordinates.
//! Port of terminaltexteffects/effects/effect_slice.py (default "vertical"
//! slice direction) adapted to this engine.

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing::EasingFn;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Easing used by the upstream effect (in_out_expo); not provided by
/// utils::easing, so defined locally.
fn in_out_expo(t: f64) -> f64 {
    if t == 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else if t < 0.5 {
        2.0_f64.powf(20.0 * t - 10.0) / 2.0
    } else {
        (2.0 - 2.0_f64.powf(-20.0 * t + 10.0)) / 2.0
    }
}

const MOVEMENT_SPEED: f64 = 0.15;
const MAX_FRAMES: usize = 5000;

pub struct Slice;

impl Slice {
    pub fn new() -> Self {
        Slice
    }
}

impl Default for Slice {
    fn default() -> Self {
        Slice::new()
    }
}

impl Effect for Slice {
    fn name(&self) -> &str {
        "slice"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let height = terminal.canvas.height as i32;
        let width = terminal.canvas.width as i32;
        let top = height;
        let bottom = 1;
        let center_column = (width + 1) / 2;

        // Final gradient (upstream defaults: 8A008A -> 00D1FF -> FFFFFF, vertical).
        let stops = [
            Color::from_hex("8A008A").unwrap_or(Color::new(0x8A, 0x00, 0x8A)),
            Color::from_hex("00D1FF").unwrap_or(Color::new(0x00, 0xD1, 0xFF)),
            Color::from_hex("FFFFFF").unwrap_or(Color::new(0xFF, 0xFF, 0xFF)),
        ];
        let gradient = Gradient::new(&stops, 12);

        // Apply the final gradient color to every character via a one-frame scene.
        for character in terminal.get_characters_mut() {
            let fraction = if height > 1 {
                (character.input_coord.row - 1) as f64 / (height - 1) as f64
            } else {
                0.0
            };
            let color = gradient.get_color_at_fraction(fraction);
            let scene = character.animation.new_scene("final_color", false);
            scene.add_frame(
                character.input_symbol,
                1,
                ColorPair::new(color, None),
                false,
            );
            character.animation.activate_scene("final_color");
        }

        // Group characters into rows, top to bottom (ids only, to avoid borrows).
        let mut rows: Vec<Vec<usize>> = Vec::new();
        for row in (1..=height).rev() {
            let ids: Vec<usize> = terminal
                .get_characters()
                .iter()
                .filter(|c| c.input_coord.row == row)
                .map(|c| c.character_id)
                .collect();
            if !ids.is_empty() {
                rows.push(ids);
            }
        }
        let num_rows = rows.len();

        let ease: EasingFn = in_out_expo;

        // Configure a character: move it to `start`, path it back to input_coord.
        let mut configure = |terminal: &mut Terminal, id: usize, start: Coord| {
            if let Some(character) = terminal
                .get_characters_mut()
                .iter_mut()
                .find(|c| c.character_id == id)
            {
                character.motion.current_coord = start;
                let input_coord = character.input_coord;
                let path = character
                    .motion
                    .new_path("input_coord", MOVEMENT_SPEED, Some(ease));
                path.add_waypoint(start);
                path.add_waypoint(input_coord);
                character.motion.activate_path("input_coord");
                character.is_visible = true;
            }
        };

        for row_index in 0..num_rows {
            // Left half of this row slides in from one row above the top.
            let left_half: Vec<(usize, i32)> = rows[row_index]
                .iter()
                .filter_map(|&id| {
                    terminal
                        .get_characters()
                        .iter()
                        .find(|c| c.character_id == id)
                        .filter(|c| c.input_coord.column <= center_column)
                        .map(|c| (id, c.input_coord.column))
                })
                .collect();
            for (id, column) in left_half {
                configure(&mut terminal, id, Coord::new(column, top + 1));
            }

            // Right half of the opposite row slides in from one row below the bottom.
            let opposite_index = num_rows - 1 - row_index;
            let right_half: Vec<(usize, i32)> = rows[opposite_index]
                .iter()
                .filter_map(|&id| {
                    terminal
                        .get_characters()
                        .iter()
                        .find(|c| c.character_id == id)
                        .filter(|c| c.input_coord.column > center_column)
                        .map(|c| (id, c.input_coord.column))
                })
                .collect();
            for (id, column) in right_half {
                configure(&mut terminal, id, Coord::new(column, bottom - 1));
            }
        }

        // Run the effect to completion, collecting frames.
        let mut frames = Vec::new();
        frames.push(terminal.get_formatted_output_string());
        for _ in 0..MAX_FRAMES {
            let active = terminal.tick();
            frames.push(terminal.get_formatted_output_string());
            if active == 0 {
                break;
            }
        }
        frames
    }
}
