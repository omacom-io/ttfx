//! Wipe effect: performs a directional wipe across the canvas, revealing the
//! input text group by group with a fade-through gradient (port of the Python
//! `effect_wipe.py`, default direction `diagonal_bottom_left_to_top_right`).

use std::collections::HashMap;

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Gradient stops matching the Python defaults ("833ab4", "fd1d1d", "fcb045").
const FINAL_GRADIENT_STOPS: [&str; 3] = ["833ab4", "fd1d1d", "fcb045"];
/// Interpolation steps between gradient stops (Python default: 12).
const FINAL_GRADIENT_STEPS: usize = 12;
/// Ticks each gradient frame is held (Python `final_gradient_frames`, default 5).
const FINAL_GRADIENT_FRAMES: u32 = 5;
/// Ticks to wait between activating wipe groups (Python `wipe_delay`, default 0).
const WIPE_DELAY: u32 = 0;
/// Hard cap so a bug can never spin forever.
const MAX_FRAMES: usize = 20_000;

pub struct Wipe;

impl Wipe {
    pub fn new() -> Self {
        Wipe
    }

    fn gradient_stops() -> Vec<Color> {
        FINAL_GRADIENT_STOPS
            .iter()
            .filter_map(|hex| Color::from_hex(hex))
            .collect()
    }
}

impl Effect for Wipe {
    fn name(&self) -> &str {
        "wipe"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let height = terminal.canvas.height as i32;

        let stops = Self::gradient_stops();
        let final_gradient = Gradient::new(&stops, FINAL_GRADIENT_STEPS);

        // Build the per-character wipe scenes: a fade from the first gradient
        // stop through to the character's final color (mapped vertically, as
        // in the Python coordinate color mapping with Direction.VERTICAL).
        for character in terminal.get_characters_mut() {
            let fraction = if height > 1 {
                (character.input_coord.row - 1) as f64 / (height - 1) as f64
            } else {
                0.0
            };
            let final_color = final_gradient
                .get_color_at_fraction(fraction)
                .or_else(|| stops.last().copied());

            let mut wipe_stops: Vec<Color> = Vec::new();
            if let Some(first) = stops.first() {
                wipe_stops.push(*first);
            }
            if let Some(color) = final_color {
                wipe_stops.push(color);
            }
            let wipe_gradient = Gradient::new(&wipe_stops, FINAL_GRADIENT_STEPS);

            let symbol = character.input_symbol;
            let scene = character.animation.new_scene("wipe", false);
            if wipe_gradient.spectrum.is_empty() {
                scene.add_frame(symbol, FINAL_GRADIENT_FRAMES, ColorPair::default(), false);
            } else {
                for color in &wipe_gradient.spectrum {
                    scene.add_frame(
                        symbol,
                        FINAL_GRADIENT_FRAMES,
                        ColorPair::fg(*color),
                        false,
                    );
                }
            }
        }

        // Group characters along diagonals running bottom-left to top-right.
        // In TTE coordinates row 1 is the bottom, so the diagonal index
        // (column + row) is smallest at the bottom-left corner and the wipe
        // sweeps toward the top-right.
        let mut diagonal_groups: HashMap<i32, Vec<usize>> = HashMap::new();
        for character in terminal.get_characters() {
            let key = character.input_coord.column + character.input_coord.row;
            diagonal_groups
                .entry(key)
                .or_default()
                .push(character.character_id);
        }
        let mut keys: Vec<i32> = diagonal_groups.keys().copied().collect();
        keys.sort_unstable();
        let mut pending_groups: Vec<Vec<usize>> = keys
            .into_iter()
            .map(|key| {
                let mut group = diagonal_groups.remove(&key).unwrap_or_default();
                group.sort_unstable();
                group
            })
            .collect();
        pending_groups.reverse(); // pop() takes from the back, so reverse for FIFO order.

        let mut frames: Vec<String> = Vec::new();
        let mut delay: u32 = 0;

        loop {
            if !pending_groups.is_empty() {
                if delay == 0 {
                    if let Some(group) = pending_groups.pop() {
                        for character_id in group {
                            terminal.set_character_visibility(character_id, true);
                            if let Some(character) = terminal
                                .get_characters_mut()
                                .iter_mut()
                                .find(|c| c.character_id == character_id)
                            {
                                character.animation.activate_scene("wipe");
                            }
                        }
                    }
                    delay = WIPE_DELAY;
                } else {
                    delay -= 1;
                }
            }

            let active = terminal.tick();
            frames.push(terminal.get_formatted_output_string());

            if pending_groups.is_empty() && active == 0 {
                break;
            }
            if frames.len() >= MAX_FRAMES {
                break;
            }
        }

        frames
    }
}
