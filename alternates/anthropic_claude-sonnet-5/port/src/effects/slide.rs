use std::collections::HashMap;

use super::Effect;
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::{lerp, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Slide: characters are grouped by row, and each row slides in
/// horizontally from off-canvas, alternating entry side (left/right) by
/// row index, with each successive row staggered by a small gap. Mirrors
/// the shape of `terminaltexteffects/effects/effect_slide.py`'s default
/// "row" grouping mode.
pub struct Slide {
    movement_speed: f64,
    gap: i32,
}

impl Slide {
    pub fn new() -> Self {
        Slide {
            movement_speed: 1.0,
            gap: 3,
        }
    }
}

/// Per-character slide state, computed once up front from the character's
/// row grouping and used to drive manual position/appearance updates each
/// frame (the provided `Motion`/`Path` stepping only snaps to a path's
/// zero-length anchor segment, so movement here is computed directly).
struct Slider {
    id: u32,
    start: Coord,
    target: Coord,
    start_tick: i32,
    duration: i32,
    color: Color,
}

impl Effect for Slide {
    fn name(&self) -> &str {
        "slide"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let canvas_width = terminal.canvas.width as i32;
        let canvas_height = terminal.canvas.height as i32;

        // Group character ids by row.
        let mut rows: HashMap<i32, Vec<u32>> = HashMap::new();
        for character in terminal.get_characters() {
            rows.entry(character.input_coord.row).or_default().push(character.id);
        }
        let mut row_keys: Vec<i32> = rows.keys().copied().collect();
        row_keys.sort_unstable();

        // Final resting gradient, mapped top row to bottom row.
        let stops = [
            Color::Rgb(0x8A, 0x00, 0x8A),
            Color::Rgb(0x00, 0xD1, 0xFF),
            Color::Rgb(0xFF, 0xFF, 0xFF),
        ];
        let gradient = Gradient::new(&stops, 12);

        let mut sliders: Vec<Slider> = Vec::new();

        for (group_index, row) in row_keys.iter().enumerate() {
            let char_ids = rows.get(row).cloned().unwrap_or_default();
            let slide_from_left = group_index % 2 == 0;
            let start_tick = group_index as i32 * self.gap;

            let color_index = if row_keys.len() > 1 && !gradient.is_empty() {
                let frac = *row as f64 / (canvas_height - 1).max(1) as f64;
                (frac * (gradient.len() - 1) as f64).round() as usize
            } else {
                0
            };
            let color = gradient.get(color_index).unwrap_or(Color::Rgb(255, 255, 255));

            for id in char_ids {
                if let Some(character) = terminal.get_character(id) {
                    let target = character.input_coord;
                    let start_col = if slide_from_left {
                        -(canvas_width) - 1
                    } else {
                        canvas_width * 2 + 1
                    };
                    let start = Coord::new(start_col, target.row);
                    let distance = (target.column - start.column).unsigned_abs().max(1) as f64;
                    let duration = ((distance / self.movement_speed).round() as i32).max(1);
                    sliders.push(Slider {
                        id,
                        start,
                        target,
                        start_tick,
                        duration,
                        color,
                    });
                }
            }
        }

        // Initialize every character at its off-screen start position.
        for slider in &sliders {
            if let Some(character) = terminal.get_character_mut(slider.id) {
                character.motion.current_pos = (slider.start.column as f64, slider.start.row as f64);
                character.motion.current_coord = slider.start;
            }
        }

        let max_end_tick = sliders.iter().map(|s| s.start_tick + s.duration).max().unwrap_or(0);

        let mut frames = Vec::new();
        for tick in 0..=max_end_tick {
            for slider in &sliders {
                if let Some(character) = terminal.get_character_mut(slider.id) {
                    if tick <= slider.start_tick {
                        character.motion.current_coord = slider.start;
                        character.motion.current_pos = (slider.start.column as f64, slider.start.row as f64);
                    } else {
                        let elapsed = (tick - slider.start_tick).min(slider.duration);
                        let t = (elapsed as f64 / slider.duration as f64).clamp(0.0, 1.0);
                        let eased_t = easing::ease_out_quad(t);
                        let (x, y) = lerp(slider.start, slider.target, eased_t);
                        character.motion.current_pos = (x, y);
                        character.motion.current_coord = Coord::new(x.round() as i32, y.round() as i32);

                        if elapsed >= slider.duration {
                            let symbol = character.input_symbol;
                            character
                                .animation
                                .set_appearance(symbol, Some(ColorPair::new(Some(slider.color), None)));
                        }
                    }
                }
            }
            frames.push(terminal.render());
        }

        frames
    }
}
