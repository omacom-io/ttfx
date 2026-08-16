//! Matrix digital rain (port of TTE's `effect_matrix.py`).
//!
//! Green symbol streams rain down every column of the canvas. After the rain
//! phase the streams drain off the bottom of the canvas and the input text
//! resolves out of the rain top-to-bottom: each character flickers through
//! random matrix symbols, flashes with the highlight color, then settles into
//! its final gradient color.

use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Symbols used for the rain streams and the resolve scramble
/// (half-width katakana plus digits/punctuation, as in the Python effect).
const RAIN_SYMBOLS: &[char] = &[
    'ｦ', 'ｱ', 'ｳ', 'ｴ', 'ｵ', 'ｶ', 'ｷ', 'ｹ', 'ｺ', 'ｻ', 'ｼ', 'ｽ', 'ｾ', 'ｿ', 'ﾀ', 'ﾂ', 'ﾃ', 'ﾅ',
    'ﾆ', 'ﾇ', 'ﾈ', 'ﾊ', 'ﾋ', 'ﾎ', 'ﾏ', 'ﾐ', 'ﾑ', 'ﾒ', 'ﾓ', 'ﾔ', 'ﾕ', 'ﾗ', 'ﾘ', 'ﾜ', '0', '1',
    '2', '3', '4', '5', '7', '8', '9', 'Z', ':', '.', '=', '*', '+', '-', '<', '>',
];

/// Ticks spent in the pure-rain phase before the text starts resolving.
const RAIN_TICKS: u32 = 240;
/// Ticks the highlight flash is held when a character finishes resolving.
const HIGHLIGHT_HOLD: u32 = 4;
/// Frames of the finished text appended at the end.
const END_HOLD: usize = 30;
/// Hard safety cap on the number of generated frames.
const MAX_FRAMES: usize = 4000;

/// Small deterministic xorshift64 PRNG so the effect is self-contained.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Inclusive integer range.
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next() % ((hi - lo + 1) as u64)) as i32
    }

    fn chance(&mut self, probability: f64) -> bool {
        ((self.next() % 10_000) as f64 / 10_000.0) < probability
    }

    fn pick(&mut self, set: &[char]) -> char {
        set[(self.next() % set.len() as u64) as usize]
    }
}

/// One falling rain stream: a bright head with a green gradient trail above it.
struct RainDrop {
    column: i32,
    head_row: i32,
    length: i32,
    fall_delay: u32,
    ticks: u32,
    head_symbol: char,
    trail: Vec<char>,
}

/// Resolution schedule for one input character.
struct TextChar {
    symbol: char,
    coord: Coord,
    resolve_start: u32,
    resolve_end: u32,
    scramble: char,
    final_color: Color,
}

pub struct Matrix;

impl Matrix {
    pub fn new() -> Self {
        Matrix
    }
}

impl Default for Matrix {
    fn default() -> Self {
        Matrix::new()
    }
}

impl Effect for Matrix {
    fn name(&self) -> &str {
        "matrix"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;

        let seed = input
            .bytes()
            .fold(0x9e37_79b9_7f4a_7c15_u64, |acc, b| {
                acc.wrapping_mul(131).wrapping_add(b as u64)
            });
        let mut rng = Rng::new(seed);

        let highlight = Color::from_hex("dadada").expect("valid hex");
        let rain_bright = Color::from_hex("92be92").expect("valid hex");
        let rain_dark = Color::from_hex("185318").expect("valid hex");
        let rain_gradient = Gradient::new(&[rain_bright, rain_dark], 12);
        let final_gradient = Gradient::new(&[rain_dark, rain_bright], 12);

        // Snapshot the input characters and schedule their resolution:
        // top rows resolve first, following the draining rain downward.
        let mut chars: Vec<TextChar> = terminal
            .get_characters()
            .iter()
            .map(|c| (c.input_symbol, c.input_coord))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(symbol, coord)| {
                let start = RAIN_TICKS
                    + ((height - coord.row).max(0) as u32) * 3
                    + rng.range(0, 12) as u32;
                let duration = rng.range(8, 24) as u32;
                let fraction = if height > 1 {
                    (coord.row - 1) as f64 / (height - 1) as f64
                } else {
                    0.0
                };
                let final_color = final_gradient
                    .get_color_at_fraction(fraction)
                    .unwrap_or(rain_bright);
                TextChar {
                    symbol,
                    coord,
                    resolve_start: start,
                    resolve_end: start + duration,
                    scramble: rng.pick(RAIN_SYMBOLS),
                    final_color,
                }
            })
            .collect();

        let end_tick = chars
            .iter()
            .map(|c| c.resolve_end)
            .max()
            .unwrap_or(RAIN_TICKS)
            + HIGHLIGHT_HOLD;

        let mut drops: Vec<RainDrop> = Vec::new();
        let mut next_spawn: Vec<u32> = (0..width).map(|_| rng.range(0, 45) as u32).collect();
        let mut frames: Vec<String> = Vec::new();
        let mut tick: u32 = 0;

        loop {
            // Spawn new drops per column during the rain phase only.
            if tick < RAIN_TICKS {
                for col in 0..width {
                    if tick >= next_spawn[col as usize] {
                        let length = rng.range(3, (height * 3 / 4).max(6));
                        let trail = (0..length).map(|_| rng.pick(RAIN_SYMBOLS)).collect();
                        drops.push(RainDrop {
                            column: col + 1,
                            head_row: height,
                            length,
                            fall_delay: rng.range(1, 3) as u32,
                            ticks: 0,
                            head_symbol: rng.pick(RAIN_SYMBOLS),
                            trail,
                        });
                        next_spawn[col as usize] = tick + rng.range(12, 50) as u32;
                    }
                }
            }

            // Advance the drops; occasionally swap a trail symbol for shimmer.
            for drop in &mut drops {
                drop.ticks += 1;
                if drop.ticks >= drop.fall_delay {
                    drop.ticks = 0;
                    drop.head_row -= 1;
                    drop.head_symbol = rng.pick(RAIN_SYMBOLS);
                }
                if !drop.trail.is_empty() && rng.chance(0.12) {
                    let idx = (rng.next() % drop.trail.len() as u64) as usize;
                    drop.trail[idx] = rng.pick(RAIN_SYMBOLS);
                }
            }
            drops.retain(|d| d.head_row + d.length >= 1);

            // Re-roll the scramble symbol of resolving characters every few ticks.
            for ch in &mut chars {
                if tick >= ch.resolve_start
                    && tick < ch.resolve_end
                    && (tick - ch.resolve_start) % 3 == 0
                {
                    ch.scramble = rng.pick(RAIN_SYMBOLS);
                }
            }

            // Render: rain first, resolving/resolved text on top.
            terminal.canvas.clear();
            for drop in &drops {
                terminal.canvas.set_cell(
                    Coord::new(drop.column, drop.head_row),
                    CharacterVisual::new(drop.head_symbol, true, ColorPair::fg(highlight)),
                );
                for (i, sym) in drop.trail.iter().enumerate() {
                    let offset = i as i32 + 1;
                    let fraction = offset as f64 / drop.length.max(1) as f64;
                    if let Some(color) = rain_gradient.get_color_at_fraction(fraction) {
                        terminal.canvas.set_cell(
                            Coord::new(drop.column, drop.head_row + offset),
                            CharacterVisual::new(*sym, false, ColorPair::fg(color)),
                        );
                    }
                }
            }
            for ch in &chars {
                if tick < ch.resolve_start {
                    continue;
                }
                let visual = if tick < ch.resolve_end {
                    CharacterVisual::new(ch.scramble, false, ColorPair::fg(rain_bright))
                } else if tick < ch.resolve_end + HIGHLIGHT_HOLD {
                    CharacterVisual::new(ch.symbol, true, ColorPair::fg(highlight))
                } else {
                    CharacterVisual::new(ch.symbol, false, ColorPair::fg(ch.final_color))
                };
                terminal.canvas.set_cell(ch.coord, visual);
            }
            frames.push(terminal.canvas.to_frame_string());

            tick += 1;
            if (tick >= end_tick && drops.is_empty()) || frames.len() >= MAX_FRAMES {
                break;
            }
        }

        // Hold the finished text for a moment.
        if let Some(last) = frames.last().cloned() {
            for _ in 0..END_HOLD {
                if frames.len() >= MAX_FRAMES {
                    break;
                }
                frames.push(last.clone());
            }
        }

        frames
    }
}
