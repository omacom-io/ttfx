//! Fireworks effect: characters are gathered into shells, launched from the
//! bottom of the canvas to a random apex, exploded outward into a ring, then
//! fall to their input coordinates while fading to the final gradient color.
//!
//! Port of terminaltexteffects/effects/effect_fireworks.py, adapted to the
//! reduced engine (no event handlers), so the phase transitions are driven
//! explicitly by a per-character state machine.

use std::f64::consts::PI;

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

const FIREWORK_SYMBOL: char = 'o';
const FIREWORK_COLORS: [&str; 5] = ["88F7E2", "44D492", "F5EB67", "FFA15C", "FA233E"];
const FINAL_GRADIENT_STOPS: [&str; 3] = ["8A008A", "00D1FF", "FFFFFF"];
/// Fraction of total characters per firework shell.
const FIREWORK_VOLUME: f64 = 0.02;
/// Explosion ring radius as a fraction of the canvas width.
const EXPLODE_DISTANCE: f64 = 0.1;
/// Ticks between successive shell launches.
const LAUNCH_DELAY: usize = 30;
/// Hard cap on simulated ticks (safety net).
const MAX_TICKS: usize = 20_000;

/// Small deterministic PRNG (splitmix-style) so the effect needs no deps.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Random i32 in `lo..=hi`; returns `lo` when the range is empty.
    fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as i32
    }

    fn range_usize(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            return lo;
        }
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as usize
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        if items.len() < 2 {
            return;
        }
        for i in (1..items.len()).rev() {
            let j = self.range_usize(0, i);
            items.swap(i, j);
        }
    }

    fn choice<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let idx = self.range_usize(0, items.len().saturating_sub(1));
        &items[idx]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Waiting,
    Launching,
    Exploding,
    Falling,
    Done,
}

/// Evenly spaced coords on a circle around `center`.
fn coords_on_circle(center: Coord, radius: i32, count: usize) -> Vec<Coord> {
    let count = count.max(1);
    (0..count)
        .map(|i| {
            let angle = 2.0 * PI * (i as f64) / (count as f64);
            Coord::new(
                center.column + (radius as f64 * angle.cos()).round() as i32,
                center.row + (radius as f64 * angle.sin()).round() as i32,
            )
        })
        .collect()
}

fn seed_from_input(input: &str) -> u64 {
    let mut seed: u64 = 0x1234_5678_9ABC_DEF1;
    for b in input.bytes() {
        seed = seed.rotate_left(7) ^ (b as u64);
        seed = seed.wrapping_mul(0x100_0000_01B3);
    }
    seed
}

pub struct Fireworks;

impl Fireworks {
    pub fn new() -> Self {
        Fireworks
    }
}

impl Default for Fireworks {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Fireworks {
    fn name(&self) -> &str {
        "fireworks"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let char_count = terminal.characters.len();
        if char_count == 0 {
            return vec![terminal.get_formatted_output_string()];
        }

        let mut rng = Rng::new(seed_from_input(input));

        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;
        let explode_radius = ((width as f64 * EXPLODE_DISTANCE).round() as i32).max(1);

        let white = Color::new(255, 255, 255);
        let firework_colors: Vec<Color> = FIREWORK_COLORS
            .iter()
            .filter_map(|hex| Color::from_hex(hex))
            .collect();
        let final_stops: Vec<Color> = FINAL_GRADIENT_STOPS
            .iter()
            .filter_map(|hex| Color::from_hex(hex))
            .collect();
        let final_gradient = Gradient::new(&final_stops, 12);

        // Group characters into shells.
        let mut indices: Vec<usize> = (0..char_count).collect();
        rng.shuffle(&mut indices);
        let shell_size = ((char_count as f64 * FIREWORK_VOLUME).round() as usize).max(1);
        let shells: Vec<Vec<usize>> = indices
            .chunks(shell_size)
            .map(|chunk| chunk.to_vec())
            .collect();

        let mut launch_tick = vec![0usize; char_count];

        for (shell_index, shell) in shells.iter().enumerate() {
            let shell_color = *rng.choice(&firework_colors);

            let apex_lo = (1 + explode_radius).min(width);
            let apex_hi = (width - explode_radius).max(apex_lo);
            let apex_col = rng.range_i32(apex_lo, apex_hi);
            let row_lo = ((height / 2) + 1).clamp(1, height);
            let apex_row = rng.range_i32(row_lo, height);
            let apex = Coord::new(apex_col, apex_row);
            let origin = Coord::new(apex_col, 1);
            let ring = coords_on_circle(apex, explode_radius, shell.len());

            for (member_index, &idx) in shell.iter().enumerate() {
                launch_tick[idx] = shell_index * LAUNCH_DELAY;

                let input_coord = terminal.characters[idx].input_coord;
                let input_symbol = terminal.characters[idx].input_symbol;
                let ring_coord = ring[member_index];

                let final_fraction = if height > 1 {
                    (input_coord.row - 1) as f64 / (height - 1) as f64
                } else {
                    0.0
                };
                let final_color = final_gradient
                    .get_color_at_fraction(final_fraction)
                    .unwrap_or(white);

                let character = &mut terminal.characters[idx];
                character.motion.current_coord = origin;

                // Motion paths for the three phases.
                let path = character
                    .motion
                    .new_path("launch", 0.5, Some(easing::out_expo));
                path.add_waypoint(origin);
                path.add_waypoint(apex);

                let path = character
                    .motion
                    .new_path("explode", 0.4, Some(easing::out_quad));
                path.add_waypoint(apex);
                path.add_waypoint(ring_coord);

                let path = character
                    .motion
                    .new_path("input", 0.3, Some(easing::in_out_cubic));
                path.add_waypoint(ring_coord);
                path.add_waypoint(input_coord);

                // Launch: twinkling shell rising to the apex.
                let scene = character.animation.new_scene("launch", true);
                scene.add_frame(FIREWORK_SYMBOL, 2, ColorPair::fg(shell_color), true);
                scene.add_frame(FIREWORK_SYMBOL, 2, ColorPair::fg(white), true);

                // Explode: bright flash fading into the shell color.
                let explode_gradient = Gradient::new(&[white, shell_color], 8);
                let scene = character.animation.new_scene("explode", false);
                for color in &explode_gradient.spectrum {
                    scene.add_frame(FIREWORK_SYMBOL, 3, ColorPair::fg(*color), false);
                }

                // Fall: input symbol fading from shell color to the final color.
                let fall_gradient = Gradient::new(&[shell_color, final_color], 10);
                let scene = character.animation.new_scene("fall", false);
                for color in &fall_gradient.spectrum {
                    scene.add_frame(input_symbol, 4, ColorPair::fg(*color), false);
                }
            }
        }

        // Simulate.
        let mut phases = vec![Phase::Waiting; char_count];
        let mut frames_out: Vec<String> = Vec::new();

        for tick in 0..MAX_TICKS {
            // Launch shells whose time has arrived.
            for idx in 0..char_count {
                if phases[idx] == Phase::Waiting && tick >= launch_tick[idx] {
                    let character = &mut terminal.characters[idx];
                    character.is_visible = true;
                    character.animation.activate_scene("launch");
                    character.motion.activate_path("launch");
                    phases[idx] = Phase::Launching;
                }
            }

            terminal.tick();

            // Phase transitions once a path finishes.
            for idx in 0..char_count {
                let character = &mut terminal.characters[idx];
                match phases[idx] {
                    Phase::Launching if character.motion.movement_is_complete() => {
                        character.animation.activate_scene("explode");
                        character.motion.activate_path("explode");
                        phases[idx] = Phase::Exploding;
                    }
                    Phase::Exploding if character.motion.movement_is_complete() => {
                        character.animation.activate_scene("fall");
                        character.motion.activate_path("input");
                        phases[idx] = Phase::Falling;
                    }
                    Phase::Falling
                        if character.motion.movement_is_complete()
                            && character.animation.active_scene_is_complete() =>
                    {
                        phases[idx] = Phase::Done;
                    }
                    _ => {}
                }
            }

            frames_out.push(terminal.get_formatted_output_string());

            if phases.iter().all(|phase| *phase == Phase::Done) {
                break;
            }
        }

        frames_out
    }
}
