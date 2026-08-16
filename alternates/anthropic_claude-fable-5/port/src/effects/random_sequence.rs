//! random_sequence: characters are revealed in random order, each fading in
//! from a starting color to its final gradient color.
//!
//! Port of terminaltexteffects/effects/effect_random_sequence.py.

use std::time::{SystemTime, UNIX_EPOCH};

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Simple xorshift64 PRNG so we do not depend on an external rand crate.
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        XorShift64 {
            state: if seed == 0 { 0x9E3779B97F4A7C15 } else { seed },
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

    /// Uniform-ish value in [0, n).
    fn gen_range(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
}

fn fisher_yates_shuffle<T>(items: &mut Vec<T>, rng: &mut XorShift64) {
    if items.len() < 2 {
        return;
    }
    for i in (1..items.len()).rev() {
        let j = rng.gen_range(i + 1);
        items.swap(i, j);
    }
}

pub struct RandomSequence {
    /// Color characters start with before fading toward their final color.
    starting_color: Color,
    /// Stops for the final gradient applied across the canvas (vertical).
    final_gradient_stops: Vec<Color>,
    /// Interpolation steps between final gradient stops.
    final_gradient_steps: usize,
    /// Ticks each fade-in frame is held for.
    final_gradient_frames: u32,
    /// Fraction of characters revealed each tick.
    speed: f64,
}

impl RandomSequence {
    pub fn new() -> Self {
        RandomSequence {
            starting_color: Color::from_hex("000000").expect("valid hex"),
            final_gradient_stops: vec![
                Color::from_hex("8A008A").expect("valid hex"),
                Color::from_hex("00D1FF").expect("valid hex"),
                Color::from_hex("FFFFFF").expect("valid hex"),
            ],
            final_gradient_steps: 12,
            final_gradient_frames: 12,
            speed: 0.004,
        }
    }
}

impl Default for RandomSequence {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for RandomSequence {
    fn name(&self) -> &str {
        "random_sequence"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let mut frames: Vec<String> = Vec::new();

        let char_count = terminal.get_characters().len();
        if char_count == 0 {
            return frames;
        }

        let height = terminal.canvas.height as i32;
        let final_gradient = Gradient::new(&self.final_gradient_stops, self.final_gradient_steps);

        // Build the fade-in scene for every character. The final color is
        // taken from a vertical gradient across the canvas (row 1 = bottom,
        // matching Gradient.Direction.VERTICAL in the Python original).
        for character in terminal.get_characters_mut() {
            let row_fraction = if height > 1 {
                (character.input_coord.row - 1) as f64 / (height - 1) as f64
            } else {
                1.0
            };
            let final_color = final_gradient
                .get_color_at_fraction(row_fraction)
                .unwrap_or(self.starting_color);

            let fade_gradient = Gradient::new(&[self.starting_color, final_color], 7);
            let scene = character.animation.new_scene("fade_in", false);
            for color in &fade_gradient.spectrum {
                scene.add_frame(
                    character.input_symbol,
                    self.final_gradient_frames,
                    ColorPair::fg(*color),
                    false,
                );
            }
        }

        // Shuffle character ids to reveal them in a random order.
        let mut pending: Vec<usize> = terminal
            .get_characters()
            .iter()
            .map(|c| c.character_id)
            .collect();
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5DEECE66D);
        let mut rng = XorShift64::new(seed);
        fisher_yates_shuffle(&mut pending, &mut rng);

        // Percentage of characters revealed per tick, at least one.
        let characters_per_tick = ((self.speed * char_count as f64) as usize).max(1);

        let mut active = 0usize;
        while !pending.is_empty() || active > 0 {
            for _ in 0..characters_per_tick {
                match pending.pop() {
                    Some(character_id) => {
                        terminal.set_character_visibility(character_id, true);
                        if let Some(character) = terminal
                            .get_characters_mut()
                            .iter_mut()
                            .find(|c| c.character_id == character_id)
                        {
                            character.animation.activate_scene("fade_in");
                        }
                    }
                    None => break,
                }
            }
            active = terminal.tick();
            frames.push(terminal.get_formatted_output_string());
        }

        frames
    }
}
