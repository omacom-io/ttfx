//! Thunderstorm effect: characters rain down from above the canvas to their
//! input coordinates while lightning strikes flash columns of the storm and
//! occasional sheet lightning washes over the whole canvas.

use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::easing::EasingFn;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Small deterministic xorshift PRNG so the effect needs no external crates.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
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

    /// Uniform float in [0, 1).
    fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform usize in [0, n). Returns 0 when n == 0.
    fn usize(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    fn chance(&mut self, p: f64) -> bool {
        self.f64() < p
    }
}

pub struct Thunderstorm;

impl Thunderstorm {
    pub fn new() -> Self {
        Thunderstorm
    }
}

impl Effect for Thunderstorm {
    fn name(&self) -> &str {
        "thunderstorm"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let height = terminal.canvas.height as i32;
        let width = terminal.canvas.width as i32;
        let mut rng = Rng::new(0x5EED_1BAD_C0FF_EE01 ^ input.len() as u64);

        // Storm palette.
        let rain_colors = [
            Color::new(0x3B, 0x5B, 0x6B),
            Color::new(0x5F, 0x8C, 0xA3),
            Color::new(0x89, 0xC4, 0xE1),
        ];
        let rain_symbols = ['|', '.', '`', ':'];
        let flash_white = Color::new(0xFF, 0xFF, 0xFF);
        let flash_yellow = Color::new(0xF5, 0xF5, 0xA0);
        let final_gradient = Gradient::new(
            &[
                Color::new(0x4C, 0x5B, 0x6B),
                Color::new(0x8E, 0xA6, 0xB8),
                Color::new(0xE6, 0xF2, 0xF8),
            ],
            12,
        );

        // Per-character setup: a fall path from above the canvas down to the
        // input coordinate, a looping "falling" raindrop scene, and a final
        // resting scene colored by a vertical storm gradient.
        for character in terminal.get_characters_mut() {
            let start_row = height + 1 + rng.usize(((height / 2).max(1)) as usize) as i32;
            let start = Coord::new(character.input_coord.column, start_row);
            character.motion.current_coord = start;

            let speed = 0.4 + rng.f64() * 0.6;
            let fall_path = character
                .motion
                .new_path("fall", speed, Some(easing::in_quad as EasingFn));
            fall_path.add_waypoint(start);
            fall_path.add_waypoint(character.input_coord);

            let falling_scn = character.animation.new_scene("falling", true);
            for _ in 0..3 {
                let symbol = rain_symbols[rng.usize(rain_symbols.len())];
                let color = rain_colors[rng.usize(rain_colors.len())];
                falling_scn.add_frame(symbol, 3, ColorPair::fg(color), false);
            }

            let fraction = if height > 1 {
                (character.input_coord.row - 1) as f64 / (height - 1) as f64
            } else {
                0.0
            };
            let final_color = final_gradient
                .get_color_at_fraction(fraction)
                .unwrap_or(flash_white);
            let final_scn = character.animation.new_scene("final", false);
            final_scn.add_frame(character.input_symbol, 1, ColorPair::fg(final_color), false);
        }

        let mut pending: Vec<usize> = terminal
            .get_characters()
            .iter()
            .map(|c| c.character_id)
            .collect();
        let mut landed = vec![false; pending.len()];
        let launch_per_frame = (pending.len() / 25).max(1);

        let mut frames: Vec<String> = Vec::new();
        let mut flash_timer: u32 = 0;
        let mut flash_column: i32 = 1;
        let mut sheet_flash: u32 = 0;

        for _ in 0..5000 {
            // Launch a random handful of pending raindrops.
            if !pending.is_empty() {
                let count = 1 + rng.usize(launch_per_frame);
                for _ in 0..count {
                    if pending.is_empty() {
                        break;
                    }
                    let idx = rng.usize(pending.len());
                    let id = pending.swap_remove(idx);
                    if let Some(ch) = terminal
                        .characters
                        .iter_mut()
                        .find(|c| c.character_id == id)
                    {
                        ch.is_visible = true;
                        ch.animation.activate_scene("falling");
                        ch.motion.activate_path("fall");
                    }
                }
            }

            let active = terminal.tick();

            // Characters that just reached their input coordinate settle into
            // their final visual.
            for ch in terminal.get_characters_mut() {
                if ch.is_visible
                    && !landed[ch.character_id]
                    && ch.motion.movement_is_complete()
                    && ch.animation.active_scene.as_deref() == Some("falling")
                {
                    landed[ch.character_id] = true;
                    ch.animation.activate_scene("final");
                }
            }

            // Occasionally start a lightning event: either a column strike or
            // a canvas-wide sheet flash.
            if flash_timer == 0 && sheet_flash == 0 && frames.len() > 10 && rng.chance(0.04) {
                if rng.chance(0.3) {
                    sheet_flash = 2;
                } else {
                    flash_timer = 5;
                    flash_column = 1 + rng.usize(width.max(1) as usize) as i32;
                }
            }

            if sheet_flash > 0 {
                sheet_flash -= 1;
                for ch in terminal.get_characters_mut() {
                    if ch.is_visible {
                        let symbol = ch.animation.current_visual.symbol;
                        ch.animation.current_visual =
                            CharacterVisual::new(symbol, true, ColorPair::fg(flash_white));
                    }
                }
            } else if flash_timer > 0 {
                flash_timer -= 1;
                let color = if flash_timer % 2 == 0 {
                    flash_white
                } else {
                    flash_yellow
                };
                let reach = 2 + (flash_timer as i32 % 2);
                for ch in terminal.get_characters_mut() {
                    if ch.is_visible
                        && (ch.motion.current_coord.column - flash_column).abs() <= reach
                    {
                        let symbol = ch.animation.current_visual.symbol;
                        ch.animation.current_visual =
                            CharacterVisual::new(symbol, true, ColorPair::fg(color));
                    }
                }
            }

            frames.push(terminal.get_formatted_output_string());

            if pending.is_empty() && active == 0 && flash_timer == 0 && sheet_flash == 0 {
                break;
            }
        }

        // Hold the settled text briefly at the end of the storm.
        if let Some(last) = frames.last().cloned() {
            for _ in 0..12 {
                frames.push(last.clone());
            }
        }

        frames
    }
}
