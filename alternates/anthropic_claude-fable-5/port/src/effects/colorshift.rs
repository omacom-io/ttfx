//! Colorshift: display a gradient that shifts its colors across the text,
//! then settles into a final gradient.
//!
//! Port of `terminaltexteffects/effects/effect_colorshift.py`. The Python
//! effect builds a looping gradient, offsets it per-character when travel is
//! enabled so the colors appear to move across the canvas, cycles through the
//! spectrum a fixed number of times, and finally fades each character from its
//! last shift color into a color drawn from the final gradient.

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair, Gradient};

// Defaults mirroring ColorShiftConfig in the Python source.
const GRADIENT_STOPS: [&str; 7] = [
    "e81416", "ffa500", "faeb36", "79c314", "487de7", "4b369d", "70369d",
];
const GRADIENT_STEPS: usize = 12;
const GRADIENT_FRAMES: u32 = 5;
const CYCLES: usize = 3;
const TRAVEL: bool = true;

const FINAL_GRADIENT_STOPS: [&str; 3] = ["833ab4", "fd1d1d", "fcb045"];
const FINAL_GRADIENT_STEPS: usize = 12;
const FINAL_GRADIENT_FRAMES: u32 = 5;

pub struct Colorshift;

impl Colorshift {
    pub fn new() -> Self {
        Colorshift
    }
}

fn parse_stops(stops: &[&str]) -> Vec<Color> {
    stops.iter().filter_map(|s| Color::from_hex(s)).collect()
}

impl Effect for Colorshift {
    fn name(&self) -> &str {
        "colorshift"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;

        // Looping shift gradient: the Python Gradient(loop=True) appends the
        // first stop to the end so the cycle wraps around seamlessly.
        let mut loop_stops = parse_stops(&GRADIENT_STOPS);
        if let Some(first) = loop_stops.first().copied() {
            loop_stops.push(first);
        }
        let shift_gradient = Gradient::new(&loop_stops, GRADIENT_STEPS);
        let spectrum_len = shift_gradient.spectrum.len();

        let final_gradient = Gradient::new(&parse_stops(&FINAL_GRADIENT_STOPS), FINAL_GRADIENT_STEPS);

        for character in terminal.get_characters_mut() {
            let symbol = character.input_symbol;

            // Travel: offset each character's spectrum by its horizontal
            // position so the colors appear to move across the text.
            let offset = if TRAVEL && width > 1 && spectrum_len > 0 {
                let fraction =
                    (character.input_coord.column - 1) as f64 / (width - 1) as f64;
                ((fraction * spectrum_len as f64).round() as usize) % spectrum_len
            } else {
                0
            };

            // Gradient scene: cycle through the shifted spectrum CYCLES times.
            let mut last_color: Option<Color> = None;
            {
                let scene = character.animation.new_scene("gradient", false);
                for _ in 0..CYCLES {
                    for i in 0..spectrum_len {
                        let color = shift_gradient.spectrum[(i + offset) % spectrum_len];
                        scene.add_frame(symbol, GRADIENT_FRAMES, ColorPair::fg(color), false);
                        last_color = Some(color);
                    }
                }
            }

            // Final scene: fade from the last shift color into the character's
            // color from the final gradient (mapped vertically, matching the
            // Python default final_gradient_direction).
            let fraction = if height > 1 {
                (character.input_coord.row - 1) as f64 / (height - 1) as f64
            } else {
                0.0
            };
            let target = final_gradient
                .get_color_at_fraction(fraction)
                .unwrap_or_else(|| Color::new(255, 255, 255));
            let fade_stops = match last_color {
                Some(color) => vec![color, target],
                None => vec![target],
            };
            let fade = Gradient::new(&fade_stops, FINAL_GRADIENT_STEPS);
            {
                let final_scene = character.animation.new_scene("final", false);
                if fade.spectrum.is_empty() {
                    final_scene.add_frame(symbol, 1, ColorPair::fg(target), false);
                } else {
                    for color in &fade.spectrum {
                        final_scene.add_frame(
                            symbol,
                            FINAL_GRADIENT_FRAMES,
                            ColorPair::fg(*color),
                            false,
                        );
                    }
                }
            }

            character.is_visible = true;
            character.animation.activate_scene("gradient");
        }

        let mut frames = Vec::new();
        frames.push(terminal.get_formatted_output_string());
        loop {
            // Once a character finishes its shift cycles, start the fade into
            // the final gradient (Python does this in __next__).
            for character in terminal.get_characters_mut() {
                if character.animation.active_scene.as_deref() == Some("gradient")
                    && character.animation.active_scene_is_complete()
                {
                    character.animation.activate_scene("final");
                }
            }

            let active = terminal.tick();
            frames.push(terminal.get_formatted_output_string());
            if active == 0 {
                break;
            }
        }
        frames
    }
}
