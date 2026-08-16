//! Middleout effect: text expands in a single row or column in the middle of the
//! canvas before expanding to its original position.
//!
//! Port of terminaltexteffects/effects/effect_middleout.py.

use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing::{self, EasingFn};
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Direction the text expands from the center line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpandDirection {
    /// Expand vertically from a center row.
    Vertical,
    /// Expand horizontally from a center column.
    Horizontal,
}

pub struct Middleout {
    /// Color for the initial text in the center of the canvas.
    starting_color: Color,
    /// Colors for the final gradient applied across the canvas (vertical direction).
    final_gradient_stops: Vec<Color>,
    /// Number of interpolation steps between gradient stops.
    final_gradient_steps: usize,
    /// Direction the text expands (Python default: "vertical").
    expand_direction: ExpandDirection,
    /// Speed of the characters moving to the center line/column.
    center_movement_speed: f64,
    /// Speed of the characters expanding to the full text.
    full_movement_speed: f64,
    /// Easing for the center movement.
    center_easing: EasingFn,
    /// Easing for the full expansion movement.
    full_easing: EasingFn,
}

impl Middleout {
    pub fn new() -> Self {
        Middleout {
            // Python defaults: starting_color=ffffff,
            // final_gradient_stops=(8A008A, 00D1FF, FFFFFF), steps=12,
            // expand_direction="vertical",
            // center_movement_speed=0.35, full_movement_speed=0.35,
            // center/full easing = in_out_sine.
            starting_color: Color::from_hex("ffffff").expect("valid hex"),
            final_gradient_stops: vec![
                Color::from_hex("8A008A").expect("valid hex"),
                Color::from_hex("00D1FF").expect("valid hex"),
                Color::from_hex("FFFFFF").expect("valid hex"),
            ],
            final_gradient_steps: 12,
            expand_direction: ExpandDirection::Vertical,
            center_movement_speed: 0.35,
            full_movement_speed: 0.35,
            center_easing: easing::in_out_sine,
            full_easing: easing::in_out_sine,
        }
    }
}

impl Default for Middleout {
    fn default() -> Self {
        Middleout::new()
    }
}

impl Effect for Middleout {
    fn name(&self) -> &str {
        "middleout"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());

        // Final gradient mapped vertically across the canvas, as the Python
        // effect's build_coordinate_color_mapping(..., direction=vertical).
        let final_gradient = Gradient::new(&self.final_gradient_stops, self.final_gradient_steps);
        let fallback_color = self.starting_color;

        let center = terminal.canvas.center();
        let canvas_height = terminal.canvas.height;

        // --- build phase (Python: MiddleoutIterator.build) ---
        for character in terminal.get_characters_mut() {
            // Final color for this character based on its input row.
            let fraction = if canvas_height > 1 {
                (character.input_coord.row - 1) as f64 / (canvas_height as f64 - 1.0)
            } else {
                0.0
            };
            let final_color = final_gradient
                .get_color_at_fraction(fraction)
                .unwrap_or(fallback_color);

            // Characters start collapsed at the canvas center.
            character.motion.current_coord = center;

            // Center path: move to the center line (vertical) or column (horizontal).
            let (column, row) = match self.expand_direction {
                ExpandDirection::Vertical => (character.input_coord.column, center.row),
                ExpandDirection::Horizontal => (center.column, character.input_coord.row),
            };
            let center_path = character.motion.new_path(
                "center",
                self.center_movement_speed,
                Some(self.center_easing),
            );
            center_path.add_waypoint(Coord::new(column, row));

            // Full path: expand out to the original input coordinate.
            let full_path =
                character
                    .motion
                    .new_path("full", self.full_movement_speed, Some(self.full_easing));
            full_path.add_waypoint(character.input_coord);

            // Full scene: fade from the starting color to the character's final
            // gradient color while it expands (Python: apply_gradient_to_symbols).
            let symbol = character.input_symbol;
            let fade = Gradient::new(&[self.starting_color, final_color], 10);
            let full_scene = character.animation.new_scene("full", false);
            for color in &fade.spectrum {
                full_scene.add_frame(symbol, 5, ColorPair::fg(*color), false);
            }

            // Show the character in the starting color and head for the center line.
            character.animation.current_visual =
                CharacterVisual::new(symbol, false, ColorPair::fg(self.starting_color));
            character.is_visible = true;
            character.motion.activate_path("center");
        }

        // --- animation loop (Python: __next__ with center/expand phases) ---
        let mut frames = Vec::new();
        frames.push(terminal.get_formatted_output_string());

        let mut expanded = false;
        const MAX_FRAMES: usize = 20_000;
        loop {
            let active = terminal.tick();
            frames.push(terminal.get_formatted_output_string());

            if active == 0 {
                if !expanded {
                    // Center phase complete: expand every character to its
                    // input coordinate while fading to the final gradient color.
                    expanded = true;
                    for character in terminal.get_characters_mut() {
                        character.motion.activate_path("full");
                        character.animation.activate_scene("full");
                    }
                } else {
                    break;
                }
            }

            if frames.len() >= MAX_FRAMES {
                break;
            }
        }

        frames
    }
}
