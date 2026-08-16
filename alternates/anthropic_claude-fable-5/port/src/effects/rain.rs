//! Rain effect: characters fall from the top of the canvas like raindrops,
//! then fade from their rain color into the final gradient color.
//!
//! Port of terminaltexteffects/effects/effect_rain.py.

use super::Effect;

use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Simple xorshift64 PRNG so we don't depend on external crates.
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E3779B97F4A7C15);
        Rng(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform integer in `0..n` (n must be > 0).
    fn gen_range(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    /// Uniform float in `[0, 1)`.
    fn gen_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Per-character progress through the effect.
#[derive(Clone, Copy, PartialEq)]
enum Stage {
    Falling,
    Fading,
}

pub struct Rain;

impl Rain {
    pub fn new() -> Self {
        Rain
    }
}

impl Default for Rain {
    fn default() -> Self {
        Rain::new()
    }
}

impl Effect for Rain {
    fn name(&self) -> &str {
        "rain"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let mut rng = Rng::new();

        // Rain colors (blues, light to dark) as in the Python defaults.
        let rain_colors: Vec<Color> = [
            "00315C", "004C8F", "0075DB", "3F91D9", "78B9F2", "9AC8F5", "B8D8F8", "E3EFFC",
        ]
        .iter()
        .filter_map(|h| Color::from_hex(h))
        .collect();

        let rain_symbols: [char; 5] = ['o', '.', ',', '*', '|'];

        // Final gradient applied vertically across the canvas.
        let final_stops: Vec<Color> = ["488bff", "b2e7de", "57eaf7"]
            .iter()
            .filter_map(|h| Color::from_hex(h))
            .collect();
        let final_gradient = Gradient::new(&final_stops, 12);

        let canvas_top = terminal.canvas.height as i32;
        let canvas_height = terminal.canvas.height;

        // --- build phase: configure every character ---
        let mut pending: Vec<usize> = Vec::new();
        let mut stages: Vec<Stage> = Vec::new();

        for idx in 0..terminal.characters.len() {
            let ch = &mut terminal.characters[idx];
            let input_symbol = ch.input_symbol;
            let input_coord = ch.input_coord;

            // Final color for this character based on its row.
            let fraction = if canvas_height > 1 {
                (input_coord.row - 1) as f64 / (canvas_height - 1) as f64
            } else {
                0.0
            };
            let final_color = final_gradient
                .get_color_at_fraction(fraction)
                .unwrap_or(Color::new(255, 255, 255));

            // Raindrop appearance: random symbol and rain color.
            let rain_color = rain_colors[rng.gen_range(rain_colors.len().max(1))];
            let rain_symbol = rain_symbols[rng.gen_range(rain_symbols.len())];

            let rain_scn = ch.animation.new_scene("rain", false);
            rain_scn.add_frame(rain_symbol, 1, ColorPair::fg(rain_color), false);

            // Fade scene: gradient from the rain color to the final color,
            // shown with the original input symbol.
            let raindrop_gradient = Gradient::new(&[rain_color, final_color], 7);
            let fade_scn = ch.animation.new_scene("fade", false);
            for color in &raindrop_gradient.spectrum {
                fade_scn.add_frame(input_symbol, 3, ColorPair::fg(*color), false);
            }

            // Start at the top of the canvas in the character's column, then
            // fall to the input coordinate at a random speed.
            let start_coord = Coord::new(input_coord.column, canvas_top);
            ch.motion.current_coord = start_coord;
            let speed = 0.1 + rng.gen_f64() * 0.1;
            let path = ch.motion.new_path("input_path", speed, Some(easing::out_quad));
            path.add_waypoint(start_coord);
            path.add_waypoint(input_coord);

            pending.push(idx);
            stages.push(Stage::Falling);
        }

        // --- run phase ---
        let mut frames: Vec<String> = Vec::new();
        let max_frames = 10_000usize;

        loop {
            // Release between 1 and 3 raindrops per frame, chosen randomly.
            if !pending.is_empty() {
                let drops = 1 + rng.gen_range(3);
                for _ in 0..drops {
                    if pending.is_empty() {
                        break;
                    }
                    let pick = rng.gen_range(pending.len());
                    let idx = pending.swap_remove(pick);
                    let ch = &mut terminal.characters[idx];
                    ch.is_visible = true;
                    ch.animation.activate_scene("rain");
                    ch.motion.activate_path("input_path");
                }
            }

            terminal.tick();

            // PATH_COMPLETE -> ACTIVATE_SCENE "fade": once a raindrop lands,
            // fade it into its final color.
            for idx in 0..terminal.characters.len() {
                if stages[idx] == Stage::Falling {
                    let ch = &mut terminal.characters[idx];
                    if ch.is_visible && ch.motion.movement_is_complete() {
                        ch.animation.activate_scene("fade");
                        stages[idx] = Stage::Fading;
                    }
                }
            }

            frames.push(terminal.get_formatted_output_string());

            let active = terminal
                .characters
                .iter()
                .filter(|c| c.is_active())
                .count();
            if (pending.is_empty() && active == 0) || frames.len() >= max_frames {
                break;
            }
        }

        // Ensure the final resting frame is rendered even for empty input.
        if frames.is_empty() {
            frames.push(terminal.get_formatted_output_string());
        }

        frames
    }
}
