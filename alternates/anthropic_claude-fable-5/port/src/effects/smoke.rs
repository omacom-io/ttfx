//! Smoke: characters rise from the bottom of the canvas as drifting wisps of
//! smoke, meandering upward before condensing into the input text.

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Symbols cycled while a character is in its "smoke" state.
const SMOKE_SYMBOLS: [char; 7] = ['.', ':', '*', 'o', '~', '°', '·'];

/// Small deterministic PRNG (splitmix-style LCG) so the effect needs no
/// external crates while still looking organic.
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg {
            state: seed | 1, // never zero
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 33) as u32
    }

    fn next_f64(&mut self) -> f64 {
        self.next_u32() as f64 / u32::MAX as f64
    }

    /// Inclusive integer range.
    fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u32() % ((hi - lo + 1) as u32)) as i32
    }

    fn range_usize(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u32() as usize) % n
        }
    }

    fn shuffle(&mut self, items: &mut [usize]) {
        if items.len() < 2 {
            return;
        }
        for i in (1..items.len()).rev() {
            let j = self.range_usize(i + 1);
            items.swap(i, j);
        }
    }
}

pub struct Smoke;

impl Smoke {
    pub fn new() -> Self {
        Smoke
    }
}

impl Default for Smoke {
    fn default() -> Self {
        Smoke::new()
    }
}

impl Effect for Smoke {
    fn name(&self) -> &str {
        "smoke"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;

        // Seed the PRNG deterministically from the input so runs are repeatable.
        let seed = input
            .bytes()
            .fold(0xC0FFEE_5EEDu64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        let mut rng = Lcg::new(seed);

        // Grays for the rising smoke, and a final gradient for the settled text.
        let smoke_gradient = Gradient::new(
            &[
                Color::new(0x3B, 0x3B, 0x3B),
                Color::new(0x7F, 0x7F, 0x7F),
                Color::new(0xBF, 0xBF, 0xBF),
                Color::new(0xEE, 0xEE, 0xEE),
            ],
            8,
        );
        let final_gradient = Gradient::new(
            &[
                Color::from_hex("8A008A").unwrap_or(Color::new(0x8A, 0x00, 0x8A)),
                Color::from_hex("00D1FF").unwrap_or(Color::new(0x00, 0xD1, 0xFF)),
                Color::from_hex("FFFFFF").unwrap_or(Color::new(0xFF, 0xFF, 0xFF)),
            ],
            10,
        );
        let settle_gray = Color::new(0xD0, 0xD0, 0xD0);

        // --- build: motion paths and animation scenes for every character ---
        for character in terminal.get_characters_mut() {
            let input_coord = character.input_coord;

            // Smoke starts near the bottom of the canvas, jittered horizontally.
            let jitter = rng.range_i32(-2, 2);
            let start_col = (input_coord.column + jitter).clamp(1, width);
            let start = Coord::new(start_col, 1);
            character.motion.current_coord = start;

            // Rising path with a few horizontal drift waypoints on the way up.
            let speed = 0.25 + rng.next_f64() * 0.45;
            let drift_count = rng.range_i32(2, 4);
            {
                let path = character.motion.new_path("rise", speed, Some(easing::out_cubic));
                path.add_waypoint(start);
                for i in 1..=drift_count {
                    let t = i as f64 / (drift_count + 1) as f64;
                    let row = start.row
                        + ((input_coord.row - start.row) as f64 * t).round() as i32;
                    let col =
                        (input_coord.column + rng.range_i32(-3, 3)).clamp(1, width);
                    path.add_waypoint(Coord::new(col, row.clamp(1, height)));
                }
                path.add_waypoint(input_coord);
            }

            // Looping smoke scene: cycle wispy symbols through the gray gradient.
            let symbol_offset = rng.range_usize(SMOKE_SYMBOLS.len());
            {
                let scene = character.animation.new_scene("smoke", true);
                let last = SMOKE_SYMBOLS.len() - 1;
                for i in 0..SMOKE_SYMBOLS.len() {
                    let symbol = SMOKE_SYMBOLS[(i + symbol_offset) % SMOKE_SYMBOLS.len()];
                    let fraction = if last == 0 { 0.0 } else { i as f64 / last as f64 };
                    let color = smoke_gradient
                        .get_color_at_fraction(fraction)
                        .unwrap_or(settle_gray);
                    scene.add_frame(symbol, 3, ColorPair::fg(color), false);
                }
            }

            // Settle scene: the smoke condenses into the real character.
            let row_fraction = if height > 1 {
                (input_coord.row - 1) as f64 / (height - 1) as f64
            } else {
                0.0
            };
            let final_color = final_gradient
                .get_color_at_fraction(row_fraction)
                .unwrap_or(settle_gray);
            {
                let scene = character.animation.new_scene("settle", false);
                scene.add_frame(character.input_symbol, 3, ColorPair::fg(settle_gray), false);
                scene.add_frame(character.input_symbol, 1, ColorPair::fg(final_color), true);
            }
        }

        // Release order: shuffled so the smoke billows up unevenly.
        let mut pending: Vec<usize> = (0..terminal.characters.len()).collect();
        rng.shuffle(&mut pending);
        let release_per_tick = (pending.len() / 25).max(1);

        // --- run: release, tick, condense, render ---
        let mut frames_out: Vec<String> = Vec::new();
        loop {
            // Release the next batch of smoke wisps.
            for _ in 0..release_per_tick {
                if let Some(idx) = pending.pop() {
                    let character = &mut terminal.characters[idx];
                    character.is_visible = true;
                    character.motion.activate_path("rise");
                    character.animation.activate_scene("smoke");
                } else {
                    break;
                }
            }

            let active = terminal.tick();

            // Characters that finished rising condense into their input symbol.
            for character in terminal.get_characters_mut() {
                if character.is_visible
                    && character.motion.movement_is_complete()
                    && character.animation.active_scene.as_deref() == Some("smoke")
                {
                    character.motion.current_coord = character.input_coord;
                    character.animation.activate_scene("settle");
                }
            }

            frames_out.push(terminal.get_formatted_output_string());

            if pending.is_empty() && active == 0 {
                break;
            }
            if frames_out.len() > 20_000 {
                break; // safety guard against runaway loops
            }
        }

        // One final frame with everything settled at rest.
        frames_out.push(terminal.get_formatted_output_string());
        frames_out
    }
}
