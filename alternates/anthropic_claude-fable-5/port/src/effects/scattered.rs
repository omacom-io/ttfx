//! Scattered effect (port of terminaltexteffects/effects/effect_scattered.py).
//!
//! Text is scattered randomly across the canvas; each character moves back to
//! its input coordinate while a color gradient plays from the first gradient
//! stop toward the character's final color (mapped vertically across the
//! canvas), mirroring the Python effect's build() logic.

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Defaults from ScatteredConfig in the Python source.
const FINAL_GRADIENT_STOPS: [&str; 3] = ["8A008A", "00D1FF", "FFFFFF"];
const FINAL_GRADIENT_STEPS: usize = 12;
const FINAL_GRADIENT_FRAMES: u32 = 12;
const MOVEMENT_SPEED: f64 = 0.5;
/// Steps for the per-character white->final gradient (Python uses steps=10).
const CHAR_GRADIENT_STEPS: usize = 10;

/// Small deterministic xorshift64* PRNG so we need no external crates.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform integer in `lo..=hi`.
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i32
    }
}

pub struct Scattered;

impl Scattered {
    pub fn new() -> Self {
        Scattered
    }
}

impl Default for Scattered {
    fn default() -> Self {
        Scattered::new()
    }
}

impl Effect for Scattered {
    fn name(&self) -> &str {
        "scattered"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;

        let stops: Vec<Color> = FINAL_GRADIENT_STOPS
            .iter()
            .filter_map(|hex| Color::from_hex(hex))
            .collect();
        let first_stop = stops.first().copied().unwrap_or(Color::new(255, 255, 255));
        let final_gradient = Gradient::new(&stops, FINAL_GRADIENT_STEPS);

        // Seed varies with input size so different texts scatter differently,
        // but the effect stays deterministic for a given input.
        let mut rng = Rng::new(0x5EED_5CA7 ^ (input.len() as u64).wrapping_mul(0x9E37_79B9));

        // --- build() ---
        for character in terminal.get_characters_mut() {
            // Final color: vertical coordinate mapping across the canvas
            // (equivalent of build_coordinate_color_mapping with VERTICAL).
            let fraction = if height > 1 {
                (character.input_coord.row - 1) as f64 / (height - 1) as f64
            } else {
                1.0
            };
            let final_color = final_gradient
                .get_color_at_fraction(fraction)
                .unwrap_or(Color::new(255, 255, 255));

            // Scatter: random start coord, or (1,1) on a degenerate canvas
            // (Python: canvas.right < 2 or canvas.top < 2).
            let start = if width < 2 || height < 2 {
                Coord::new(1, 1)
            } else {
                Coord::new(rng.range(1, width), rng.range(1, height))
            };
            character.motion.current_coord = start;

            // Path back to the input coordinate. Python eases with in_out_back,
            // which this engine lacks; in_out_cubic is the closest available.
            let path = character.motion.new_path(
                "input_coord",
                MOVEMENT_SPEED,
                Some(easing::in_out_cubic),
            );
            path.add_waypoint(character.input_coord);
            character.motion.activate_path("input_coord");

            // Gradient scene: first stop -> final color, each frame held for
            // FINAL_GRADIENT_FRAMES ticks.
            let char_gradient = Gradient::new(&[first_stop, final_color], CHAR_GRADIENT_STEPS);
            let scene = character.animation.new_scene("gradient", false);
            if char_gradient.spectrum.is_empty() {
                scene.add_frame(
                    character.input_symbol,
                    FINAL_GRADIENT_FRAMES,
                    ColorPair::fg(final_color),
                    false,
                );
            } else {
                for color in &char_gradient.spectrum {
                    scene.add_frame(
                        character.input_symbol,
                        FINAL_GRADIENT_FRAMES,
                        ColorPair::fg(*color),
                        false,
                    );
                }
            }
            character.animation.activate_scene("gradient");
            character.is_visible = true;
        }

        // --- run the effect to completion ---
        let mut frames = Vec::new();
        frames.push(terminal.get_formatted_output_string());
        const MAX_FRAMES: usize = 10_000;
        loop {
            let active = terminal.tick();
            frames.push(terminal.get_formatted_output_string());
            if active == 0 || frames.len() >= MAX_FRAMES {
                break;
            }
        }
        frames
    }
}
