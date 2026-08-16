//! Characters drop from the top of the canvas as bouncy balls and settle into the input text.

use std::collections::HashMap;

use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{find_length_of_line, lerp_coord, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

const BALL_DELAY: u32 = 7;
const MOVEMENT_SPEED: f64 = 0.25;
const HOLD_FRAMES: usize = 12;
const MAX_FRAMES: usize = 20_000;
const BALL_SYMBOLS: [&str; 5] = ["*", "o", "O", "0", "."];

fn ball_palette() -> [Color; 3] {
    [
        Color::rgb(0xd1, 0xf4, 0xa5),
        Color::rgb(0x96, 0xe2, 0xa4),
        Color::rgb(0x5a, 0xcd, 0xa9),
    ]
}

fn final_stops() -> [Color; 2] {
    [
        Color::rgb(0xf8, 0xff, 0xae),
        Color::rgb(0x43, 0xc6, 0xac),
    ]
}

/// Python `easing.out_bounce`.
fn out_bounce(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    let n1 = 7.5625;
    let d1 = 2.75;
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed | 1,
        }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        (x >> 32) as u32
    }

    fn gen_range(&mut self, low: usize, high: usize) -> usize {
        if high <= low {
            return low;
        }
        low + (self.next_u32() as usize) % (high - low)
    }
}

fn hash_input(input: &str) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for b in input.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

struct Ball {
    id: CharacterId,
    input_symbol: String,
    start: Coord,
    end: Coord,
    progress: f64,
    increment: f64,
    ball_symbol: String,
    ball_color: Color,
    final_color: Color,
    dropped: bool,
    landed: bool,
}

pub struct Bouncyballs;

impl Bouncyballs {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Bouncyballs {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Bouncyballs {
    fn name(&self) -> &str {
        "bouncyballs"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_input(input, TerminalConfig::default());
        let canvas_top = terminal.canvas.top;

        let meta: Vec<(CharacterId, String, Coord)> = terminal
            .get_characters()
            .iter()
            .map(|ch| (ch.id, ch.input_symbol.clone(), ch.input_coord))
            .collect();

        if meta.is_empty() {
            return vec![terminal.render_frame()];
        }

        let min_col = meta.iter().map(|(_, _, c)| c.column).min().unwrap_or(1);
        let max_col = meta.iter().map(|(_, _, c)| c.column).max().unwrap_or(1);
        let min_row = meta.iter().map(|(_, _, c)| c.row).min().unwrap_or(1);
        let max_row = meta.iter().map(|(_, _, c)| c.row).max().unwrap_or(1);
        let span = (max_col - min_col + max_row - min_row).max(1) as f64;

        let gradient = Gradient::new(&final_stops(), 12);
        let fallback = final_stops()[0];
        let palette = ball_palette();
        let mut rng = Rng::new(hash_input(input));

        let mut states: Vec<Ball> = Vec::with_capacity(meta.len());
        let mut groups: HashMap<i32, Vec<CharacterId>> = HashMap::new();

        for (id, input_symbol, input_coord) in meta {
            let progress = (input_coord.column - min_col + input_coord.row - min_row) as f64 / span;
            let final_color = gradient.mapped_color(progress).unwrap_or(fallback);
            let ball_symbol = BALL_SYMBOLS[rng.gen_range(0, BALL_SYMBOLS.len())].to_string();
            let ball_color = palette[rng.gen_range(0, palette.len())];
            let start = Coord {
                column: input_coord.column,
                row: canvas_top,
            };
            let length = find_length_of_line(start, input_coord);
            let increment = if length <= f64::EPSILON {
                1.0
            } else {
                MOVEMENT_SPEED / length
            };
            if let Some(ch) = terminal.get_character_mut(id) {
                ch.motion.current_coord = start;
                ch.animation
                    .set_appearance(&ball_symbol, Some(ColorPair::fg(ball_color)));
            }
            groups.entry(input_coord.row).or_default().push(id);
            states.push(Ball {
                id,
                input_symbol,
                start,
                end: input_coord,
                progress: 0.0,
                increment,
                ball_symbol,
                ball_color,
                final_color,
                dropped: false,
                landed: false,
            });
        }

        let mut frames = Vec::new();
        let mut delay = 0u32;
        let mut hold = 0usize;

        loop {
            if !groups.is_empty() {
                if delay == 0 {
                    let keys: Vec<i32> = groups.keys().copied().collect();
                    let key = keys[rng.gen_range(0, keys.len())];
                    let drop_ids: Vec<CharacterId> = {
                        let row = groups.get_mut(&key).expect("key from keys()");
                        let n = rng.gen_range(1, 6).min(row.len());
                        row.drain(0..n).collect()
                    };
                    if groups.get(&key).is_some_and(|row| row.is_empty()) {
                        groups.remove(&key);
                    }
                    for id in drop_ids {
                        if let Some(ball) = states.iter_mut().find(|ball| ball.id == id) {
                            ball.dropped = true;
                        }
                        terminal.set_character_visibility(id, true);
                    }
                }
                delay += 1;
                if delay >= BALL_DELAY {
                    delay = 0;
                }
            }

            for ball in &mut states {
                if !ball.dropped || ball.landed {
                    continue;
                }
                ball.progress = (ball.progress + ball.increment).min(1.0);
                let coord = lerp_coord(ball.start, ball.end, out_bounce(ball.progress));
                let landed_now = ball.progress >= 1.0;
                let symbol = if landed_now {
                    ball.input_symbol.clone()
                } else {
                    ball.ball_symbol.clone()
                };
                let color = if landed_now {
                    ball.final_color
                } else {
                    ball.ball_color
                };
                if landed_now {
                    ball.landed = true;
                }
                if let Some(ch) = terminal.get_character_mut(ball.id) {
                    ch.motion.current_coord = coord;
                    ch.animation
                        .set_appearance(&symbol, Some(ColorPair::fg(color)));
                }
            }

            frames.push(terminal.render_frame());

            let falling = states.iter().any(|ball| ball.dropped && !ball.landed);
            if groups.is_empty() && !falling {
                hold += 1;
                if hold >= HOLD_FRAMES {
                    break;
                }
            }
            if frames.len() >= MAX_FRAMES {
                break;
            }
        }

        frames
    }
}
