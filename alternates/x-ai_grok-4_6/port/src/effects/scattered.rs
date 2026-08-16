//! Scattered effect — characters fly into place from random canvas positions.

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{find_length_of_line, lerp_coord, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Default movement speed from the Python `ScatteredConfig`.
const MOVEMENT_SPEED: f64 = 0.3;
/// Default `--final-gradient-steps` from the Python config.
const GRADIENT_STEPS: usize = 12;
const MAX_FRAMES: usize = 10_000;

/// Move the characters into position from random starting locations.
pub struct Scattered;

impl Scattered {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Scattered {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Scattered {
    fn name(&self) -> &str {
        "scattered"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        if term.character_count() == 0 {
            return vec![term.render_frame()];
        }

        let ends: Vec<Coord> = term
            .get_characters()
            .iter()
            .map(|ch| ch.input_coord)
            .collect();

        let text_bottom = ends.iter().map(|c| c.row).min().unwrap_or(1);
        let text_top = ends.iter().map(|c| c.row).max().unwrap_or(1);
        let row_span = text_top - text_bottom;

        // Python defaults: --final-gradient-stops 8A008A 00D1FF FFFFFF
        let gradient = Gradient::new(
            &[
                Color::rgb(0x8A, 0x00, 0x8A),
                Color::rgb(0x00, 0xD1, 0xFF),
                Color::rgb(0xFF, 0xFF, 0xFF),
            ],
            GRADIENT_STEPS,
        );

        let left = term.canvas.left;
        let right = term.canvas.right;
        let bottom = term.canvas.bottom;
        let top = term.canvas.top;

        let mut rng = Lcg::new(fnv1a64(input));
        let mut starts = Vec::with_capacity(ends.len());
        let mut distances = Vec::with_capacity(ends.len());

        for (ch, &end) in term.get_characters_mut().iter_mut().zip(ends.iter()) {
            let start = Coord::new(rng.inclusive(left, right), rng.inclusive(bottom, top));
            ch.motion.current_coord = start;
            ch.is_visible = true;

            // Vertical mapping: first stop at text_bottom, last stop at text_top.
            let progress = if row_span == 0 {
                1.0
            } else {
                f64::from(end.row - text_bottom) / f64::from(row_span)
            };
            if let Some(color) = gradient.mapped_color(progress) {
                ch.animation
                    .set_appearance(&ch.input_symbol, Some(ColorPair::fg(color)));
            }

            distances.push(find_length_of_line(start, end));
            starts.push(start);
        }

        let n = ends.len();
        let mut traveled = vec![0.0_f64; n];
        let mut frames = Vec::new();

        loop {
            let mut active = false;
            {
                let chars = term.get_characters_mut();
                for i in 0..n {
                    if traveled[i] < distances[i] {
                        traveled[i] += MOVEMENT_SPEED;
                        active = true;
                    }
                    let t = if distances[i] <= 0.0 {
                        1.0
                    } else {
                        in_out_back((traveled[i] / distances[i]).clamp(0.0, 1.0))
                    };
                    chars[i].motion.current_coord = lerp_coord(starts[i], ends[i], t);
                }
            }
            if !active && !frames.is_empty() {
                break;
            }
            frames.push(term.render_frame());
            if !active || frames.len() >= MAX_FRAMES {
                break;
            }
        }

        frames
    }
}

/// Python `easing.in_out_back` (standard back-ease with overshoot).
fn in_out_back(t: f64) -> f64 {
    const C1: f64 = 1.70158;
    const C2: f64 = C1 * 1.525;
    if t < 0.5 {
        let x = 2.0 * t;
        (x * x * ((C2 + 1.0) * x - C2)) / 2.0
    } else {
        let x = 2.0 * t - 2.0;
        (x * x * ((C2 + 1.0) * x + C2) + 2.0) / 2.0
    }
}

fn fnv1a64(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.0 >> 32) as u32
    }

    /// Inclusive integer range, matching `random.randint(lo, hi)`.
    fn inclusive(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let span = (i64::from(hi) - i64::from(lo) + 1) as u32;
        lo.saturating_add((self.next_u32() % span) as i32)
    }
}
