//! Spray effect: characters are sprayed onto the canvas from a single origin
//! point, travelling to their input coordinates while shifting color from a
//! random spray color to their final gradient color.
//!
//! Port of terminaltexteffects/effects/effect_spray.py (defaults: east spray
//! position, movement speed uniform in [0.4, 1.0], out_expo easing, final
//! gradient 8A008A -> 00D1FF -> FFFFFF with 12 steps, vertical direction,
//! spray volume 0.005).

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Small deterministic xorshift64 PRNG (the crate has no rand dependency).
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Rng {
            state: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform float in [lo, hi), mirroring Python's random.uniform.
    fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_f64()
    }

    /// Inclusive integer range, mirroring Python's random.randint.
    fn randint(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            lo
        } else {
            lo + (self.next_u64() % (hi - lo + 1) as u64) as usize
        }
    }

    /// Mirror of Python's random.choice.
    fn choice<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.randint(0, items.len() - 1)]
    }

    /// Fisher-Yates, mirroring Python's random.shuffle.
    fn shuffle<T>(&mut self, items: &mut [T]) {
        if items.len() < 2 {
            return;
        }
        for i in (1..items.len()).rev() {
            let j = self.randint(0, i);
            items.swap(i, j);
        }
    }
}

/// Sprays the characters from a single point (east edge by default).
pub struct Spray {
    /// Fraction of total characters that may spawn per frame (min 1).
    spray_volume: f64,
    /// Range for per-character movement speed.
    movement_speed: (f64, f64),
    /// Interpolation steps between the final gradient stops.
    final_gradient_steps: usize,
}

impl Spray {
    pub fn new() -> Self {
        Spray {
            spray_volume: 0.005,
            movement_speed: (0.4, 1.0),
            final_gradient_steps: 12,
        }
    }
}

impl Default for Spray {
    fn default() -> Self {
        Spray::new()
    }
}

impl Effect for Spray {
    fn name(&self) -> &str {
        "spray"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let mut rng = Rng::new(0x5EED_5EED_5EED_5EED);

        let stops = [
            Color::from_hex("8A008A").expect("valid hex"),
            Color::from_hex("00D1FF").expect("valid hex"),
            Color::from_hex("FFFFFF").expect("valid hex"),
        ];
        let final_gradient = Gradient::new(&stops, self.final_gradient_steps);

        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;
        // Default spray position: east edge, vertically centered
        // (Python: Coord(canvas.right - 1, canvas.top // 2)).
        let origin = Coord::new((width - 1).max(1), (height / 2).max(1));

        // --- build: give every character a path from the origin to its input
        // coord plus a droplet color scene; activation is deferred to spawn
        // time so unspawned characters stay inert.
        let mut pending: Vec<usize> = Vec::new();
        for character in terminal.get_characters_mut() {
            // Vertical final-gradient direction: color by row fraction.
            let fraction = if height > 1 {
                (character.input_coord.row - 1) as f64 / (height - 1) as f64
            } else {
                1.0
            };
            let final_color = final_gradient
                .get_color_at_fraction(fraction)
                .unwrap_or(stops[stops.len() - 1]);

            character.motion.current_coord = origin;

            let speed = rng.uniform(self.movement_speed.0, self.movement_speed.1);
            let path = character
                .motion
                .new_path("input_coord", speed, Some(easing::out_expo));
            path.add_waypoint(character.input_coord);

            // Droplet scene: random spectrum color fading into the final color.
            let start_color = *rng.choice(&final_gradient.spectrum);
            let spray_gradient = Gradient::new(&[start_color, final_color], 25);
            let scene = character.animation.new_scene("droplet", false);
            for color in &spray_gradient.spectrum {
                scene.add_frame(character.input_symbol, 3, ColorPair::fg(*color), false);
            }

            pending.push(character.character_id);
        }
        rng.shuffle(&mut pending);

        let volume = ((pending.len() as f64 * self.spray_volume) as usize).max(1);

        // --- run: spawn a random handful each frame, tick, render.
        let mut frames: Vec<String> = Vec::new();
        let mut safety = 0usize;
        loop {
            if !pending.is_empty() {
                let count = rng.randint(1, volume);
                for _ in 0..count {
                    let Some(id) = pending.pop() else { break };
                    if let Some(character) = terminal
                        .characters
                        .iter_mut()
                        .find(|c| c.character_id == id)
                    {
                        character.animation.activate_scene("droplet");
                        character.motion.activate_path("input_coord");
                    }
                    terminal.set_character_visibility(id, true);
                }
            }

            let active = terminal.tick();
            frames.push(terminal.get_formatted_output_string());

            safety += 1;
            if (pending.is_empty() && active == 0) || safety > 20_000 {
                break;
            }
        }

        frames
    }
}
