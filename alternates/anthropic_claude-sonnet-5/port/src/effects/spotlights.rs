use super::Effect;
use crate::engine::terminal::Terminal;
use crate::utils::geometry::{self, Coord};
use crate::utils::graphics::{Color, ColorPair};

use std::collections::HashSet;

/// Simple deterministic xorshift64-based PRNG so spotlight movement is
/// reproducible without depending on an external `rand` crate (none is
/// available to this skeleton).
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        // Avoid a zero state, which would stall xorshift.
        Lcg { state: seed.wrapping_mul(2685821657736338717).wrapping_add(1) | 1 }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn gen_range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let range = (hi - lo + 1) as u64;
        lo + (self.next_u64() % range) as i32
    }
}

/// A moving spotlight: a position that eases toward a sequence of random
/// waypoints within the canvas bounds, looping once it reaches the end.
struct SpotlightState {
    pos: (f64, f64),
    waypoints: Vec<Coord>,
    current_idx: usize,
    speed: f64,
}

impl SpotlightState {
    fn advance(&mut self) {
        if self.waypoints.is_empty() {
            return;
        }
        let target = self.waypoints[self.current_idx];
        let dx = target.column as f64 - self.pos.0;
        let dy = target.row as f64 - self.pos.1;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist <= self.speed || dist == 0.0 {
            self.pos = (target.column as f64, target.row as f64);
            self.current_idx = (self.current_idx + 1) % self.waypoints.len();
        } else {
            self.pos.0 += dx / dist * self.speed;
            self.pos.1 += dy / dist * self.speed;
        }
    }
}

/// Spotlights effect: a handful of spotlights wander the canvas, revealing
/// characters that fall within their radius. Once revealed, a character
/// stays visible at a dimmer color; while actively under a spotlight it is
/// shown at a bright highlight color. Mirrors the shape of
/// `terminaltexteffects/effects/effect_spotlights.py`, simplified to the
/// primitives available in this port (no Path/Motion easing hookup needed
/// since spotlights are not `EffectCharacter`s themselves).
pub struct Spotlights {
    pub num_spotlights: usize,
    pub radius: i32,
    pub speed: f64,
    pub illuminate_color: Color,
    pub revealed_color: Color,
}

impl Spotlights {
    pub fn new() -> Self {
        Spotlights {
            num_spotlights: 3,
            radius: 6,
            speed: 1.5,
            illuminate_color: Color::Rgb(255, 255, 255),
            revealed_color: Color::Rgb(180, 180, 180),
        }
    }
}

impl Effect for Spotlights {
    fn name(&self) -> &str {
        "spotlights"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let canvas_width = terminal.canvas.width as i32;
        let canvas_height = terminal.canvas.height as i32;

        // Everything starts hidden; spotlights reveal characters as they pass.
        let ids: Vec<u32> = terminal.get_characters().iter().map(|c| c.id).collect();
        for id in &ids {
            terminal.set_character_visibility(*id, false);
        }

        let mut rng = Lcg::new(0xC0FFEE_u64);
        let mut spotlights: Vec<SpotlightState> = Vec::new();
        for i in 0..self.num_spotlights {
            let start = Coord::new(
                rng.gen_range(0, canvas_width.max(1) - 1),
                rng.gen_range(0, canvas_height.max(1) - 1),
            );
            let mut waypoints = Vec::new();
            for _ in 0..6 {
                waypoints.push(Coord::new(
                    rng.gen_range(0, canvas_width.max(1) - 1),
                    rng.gen_range(0, canvas_height.max(1) - 1),
                ));
            }
            spotlights.push(SpotlightState {
                pos: (start.column as f64, start.row as f64),
                waypoints,
                current_idx: 0,
                speed: self.speed + (i as f64) * 0.15,
            });
        }

        let total_frames = (((canvas_width + canvas_height).max(1) as usize) * 6).clamp(60, 400);
        let mut revealed: HashSet<u32> = HashSet::new();
        let mut frames = Vec::with_capacity(total_frames);

        for _ in 0..total_frames {
            for spotlight in spotlights.iter_mut() {
                spotlight.advance();
            }

            let spotlight_positions: Vec<Coord> = spotlights
                .iter()
                .map(|s| Coord::new(s.pos.0.round() as i32, s.pos.1.round() as i32))
                .collect();

            for character in terminal.get_characters_mut() {
                let mut illuminated = false;
                for spot in &spotlight_positions {
                    if geometry::distance(*spot, character.input_coord) <= self.radius as f64 {
                        illuminated = true;
                        break;
                    }
                }

                if illuminated {
                    revealed.insert(character.id);
                    character.set_visibility(true);
                    character.animation.set_appearance(
                        character.input_symbol,
                        Some(ColorPair::new(Some(self.illuminate_color), None)),
                    );
                } else if revealed.contains(&character.id) {
                    character.set_visibility(true);
                    character.animation.set_appearance(
                        character.input_symbol,
                        Some(ColorPair::new(Some(self.revealed_color), None)),
                    );
                } else {
                    character.set_visibility(false);
                }
            }

            frames.push(terminal.render());
        }

        frames
    }
}
