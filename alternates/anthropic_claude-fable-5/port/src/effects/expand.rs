//! Expand effect: every character starts at the canvas center and travels
//! outward to its input coordinate while a color gradient plays over it.
//!
//! Port of terminaltexteffects/effects/effect_expand.py:
//!   - final_gradient_stops: ("8A008A", "00D1FF", "FFFFFF"), steps=12,
//!     direction=vertical, final_gradient_frames=5
//!   - movement_speed: 0.35
//!   - expand_easing: in_out_quart upstream; the closest easing available in
//!     this port is in_out_cubic.

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::graphics::{Color, ColorPair, Gradient};

const MOVEMENT_SPEED: f64 = 0.35;
const FINAL_GRADIENT_STEPS: usize = 12;
const FINAL_GRADIENT_FRAMES: u32 = 5;
const CHARACTER_GRADIENT_STEPS: usize = 10;

pub struct Expand;

impl Expand {
    pub fn new() -> Self {
        Expand
    }
}

impl Default for Expand {
    fn default() -> Self {
        Expand::new()
    }
}

impl Effect for Expand {
    fn name(&self) -> &str {
        "expand"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let center = terminal.canvas.center();
        let height = terminal.canvas.height;

        // Final gradient (vertical direction: color determined by row).
        let stops = [
            Color::from_hex("8A008A").expect("valid hex"),
            Color::from_hex("00D1FF").expect("valid hex"),
            Color::from_hex("FFFFFF").expect("valid hex"),
        ];
        let final_gradient = Gradient::new(&stops, FINAL_GRADIENT_STEPS);

        for character in terminal.get_characters_mut() {
            // Final color mapped by the character's input row (vertical direction).
            let fraction = if height > 1 {
                (character.input_coord.row - 1) as f64 / (height - 1) as f64
            } else {
                0.0
            };
            let final_color = final_gradient
                .get_color_at_fraction(fraction)
                .unwrap_or(stops[stops.len() - 1]);

            // Start at the canvas center and build a path back to the
            // input coordinate. The origin waypoint stands in for the
            // implicit origin segment of the Python engine.
            let input_coord = character.input_coord;
            character.motion.current_coord = center;
            let path = character.motion.new_path(
                "input_coord",
                MOVEMENT_SPEED,
                Some(easing::in_out_cubic),
            );
            path.add_waypoint(center);
            path.add_waypoint(input_coord);

            // Gradient scene: from the first gradient stop to this
            // character's final color.
            let symbol = character.input_symbol;
            let char_gradient = Gradient::new(&[stops[0], final_color], CHARACTER_GRADIENT_STEPS);
            let scene = character.animation.new_scene("gradient", false);
            for color in &char_gradient.spectrum {
                scene.add_frame(symbol, FINAL_GRADIENT_FRAMES, ColorPair::fg(*color), false);
            }

            character.animation.activate_scene("gradient");
            character.motion.activate_path("input_coord");
            character.is_visible = true;
        }

        // Run the simulation until every character has finished its
        // animation and movement, collecting one frame per tick.
        let mut frames = Vec::new();
        frames.push(terminal.get_formatted_output_string());
        while terminal.tick() > 0 {
            frames.push(terminal.get_formatted_output_string());
        }
        // Capture the settled final state.
        frames.push(terminal.get_formatted_output_string());
        frames
    }
}
