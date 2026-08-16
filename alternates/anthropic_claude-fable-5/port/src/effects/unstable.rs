//! Unstable: characters are jumbled to swapped positions, rumble with growing
//! intensity, explode outward to the canvas edges, then reassemble into the
//! original input text while shifting to the final gradient colors.
//!
//! Port of terminaltexteffects/effects/effect_unstable.py, adapted to the
//! simplified engine in this crate (no event handlers; phases are driven
//! directly from the frame loop).

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Small deterministic xorshift64 PRNG (the crate has no rand dependency).
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // Never allow the zero state.
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

    /// Inclusive range [lo, hi].
    fn gen_range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i32
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        if items.len() < 2 {
            return;
        }
        for i in (1..items.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            items.swap(i, j);
        }
    }
}

const EXPLOSION_SPEED: f64 = 0.75;
const REASSEMBLY_SPEED: f64 = 0.75;
const RUMBLE_TICKS: u32 = 110;
const PAUSE_TICKS: u32 = 12;
const FINAL_HOLD_TICKS: u32 = 10;
const GUARD_LIMIT: u32 = 5000;

pub struct Unstable;

impl Unstable {
    pub fn new() -> Self {
        Unstable
    }
}

impl Effect for Unstable {
    fn name(&self) -> &str {
        "unstable"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;

        // Deterministic seed derived from the input text.
        let mut seed: u64 = 0xA5A5_1234_5678_9ABC;
        for b in input.bytes() {
            seed = seed.wrapping_mul(31).wrapping_add(b as u64);
        }
        let mut rng = Rng::new(seed);

        // Colors mirroring the Python defaults.
        let unstable_color = Color::from_hex("ff9200").expect("valid hex");
        let dim_unstable = Color::new(0x71, 0x40, 0x00);
        let final_gradient = Gradient::new(
            &[
                Color::from_hex("8A008A").expect("valid hex"),
                Color::from_hex("00D1FF").expect("valid hex"),
                Color::from_hex("FFFFFF").expect("valid hex"),
            ],
            12,
        );
        let rumble_gradient = Gradient::new(&[dim_unstable, unstable_color], 6);

        // Jumble: each character takes another character's input coordinate.
        let mut jumbled: Vec<Coord> = terminal
            .characters
            .iter()
            .map(|c| c.input_coord)
            .collect();
        rng.shuffle(&mut jumbled);

        // Build paths and scenes per character.
        for (i, character) in terminal.characters.iter_mut().enumerate() {
            let jumbled_coord = jumbled[i];
            let input_coord = character.input_coord;
            let symbol = character.input_symbol;

            character.is_visible = true;
            character.motion.current_coord = jumbled_coord;

            // Random point on one of the four canvas edges (explosion target).
            let pos = rng.gen_range(0, 3);
            let (col, row) = match pos {
                0 => (1, rng.gen_range(1, height)),
                1 => (width, rng.gen_range(1, height)),
                2 => (rng.gen_range(1, width), 1),
                _ => (rng.gen_range(1, width), height),
            };
            let edge_coord = Coord::new(col, row);

            // Explosion: jumbled position -> edge of canvas.
            let path = character
                .motion
                .new_path("explosion", EXPLOSION_SPEED, Some(easing::out_expo));
            path.add_waypoint(jumbled_coord);
            path.add_waypoint(edge_coord);

            // Reassembly: edge of canvas -> original input position.
            let path = character
                .motion
                .new_path("reassembly", REASSEMBLY_SPEED, Some(easing::out_expo));
            path.add_waypoint(edge_coord);
            path.add_waypoint(input_coord);

            // Rumble scene: looping ping-pong flicker through the unstable colors.
            let scene = character.animation.new_scene("rumble", true);
            for color in rumble_gradient
                .spectrum
                .iter()
                .chain(rumble_gradient.spectrum.iter().rev())
            {
                scene.add_frame(symbol, 2, ColorPair::fg(*color), false);
            }

            // Explosion scene: bright unstable color while flying outward.
            let scene = character.animation.new_scene("explosion", false);
            scene.add_frame(symbol, 1, ColorPair::fg(unstable_color), true);

            // Final scene: fade from unstable color to the final gradient color
            // (vertical gradient across the canvas, matching upstream defaults).
            let fraction = if height > 1 {
                (input_coord.row - 1) as f64 / (height - 1) as f64
            } else {
                0.0
            };
            let final_color = final_gradient
                .get_color_at_fraction(fraction)
                .unwrap_or(unstable_color);
            let reassembly_gradient = Gradient::new(&[unstable_color, final_color], 10);
            let scene = character.animation.new_scene("final", false);
            for color in &reassembly_gradient.spectrum {
                scene.add_frame(symbol, 3, ColorPair::fg(*color), false);
            }

            character.animation.activate_scene("rumble");
        }

        let mut frames_out: Vec<String> = Vec::new();
        frames_out.push(terminal.get_formatted_output_string());

        // Phase 1: rumble — jitter around the jumbled coords, intensifying.
        for tick in 0..RUMBLE_TICKS {
            let intensity = if tick > RUMBLE_TICKS * 3 / 4 { 2 } else { 1 };
            let jitter_now = tick % 2 == 0;
            for (i, character) in terminal.characters.iter_mut().enumerate() {
                let base = jumbled[i];
                let (dc, dr) = if jitter_now {
                    (
                        rng.gen_range(-intensity, intensity),
                        rng.gen_range(-intensity, intensity),
                    )
                } else {
                    (0, 0)
                };
                let column = (base.column + dc).clamp(1, width);
                let row = (base.row + dr).clamp(1, height);
                character.motion.current_coord = Coord::new(column, row);
            }
            terminal.tick();
            frames_out.push(terminal.get_formatted_output_string());
        }

        // Phase 2: explosion — fling every character to its edge coordinate.
        for (i, character) in terminal.characters.iter_mut().enumerate() {
            character.motion.current_coord = jumbled[i];
            character.animation.activate_scene("explosion");
            character.motion.activate_path("explosion");
        }
        let mut guard = 0u32;
        loop {
            terminal.tick();
            frames_out.push(terminal.get_formatted_output_string());
            guard += 1;
            let movement_done = terminal
                .characters
                .iter()
                .all(|c| c.motion.movement_is_complete());
            if movement_done || guard > GUARD_LIMIT {
                break;
            }
        }

        // Brief pause at the edges before reassembly.
        for _ in 0..PAUSE_TICKS {
            terminal.tick();
            frames_out.push(terminal.get_formatted_output_string());
        }

        // Phase 3: reassembly — return to input coords with the final colors.
        for character in terminal.characters.iter_mut() {
            character.animation.activate_scene("final");
            character.motion.activate_path("reassembly");
        }
        let mut guard = 0u32;
        loop {
            let active = terminal.tick();
            frames_out.push(terminal.get_formatted_output_string());
            guard += 1;
            if active == 0 || guard > GUARD_LIMIT {
                break;
            }
        }

        // Hold the finished text on screen briefly.
        for _ in 0..FINAL_HOLD_TICKS {
            terminal.tick();
            frames_out.push(terminal.get_formatted_output_string());
        }

        frames_out
    }
}
