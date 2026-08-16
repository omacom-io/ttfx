//! Overflow effect: input rows scroll the canvas out of order, then settle.

use std::collections::{BTreeMap, HashMap};

use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

const OVERFLOW_SPEED: usize = 3;
const OVERFLOW_CYCLES_LO: i32 = 2;
const OVERFLOW_CYCLES_HI: i32 = 4;
const COLOR_HOLD: usize = 16;
const FINAL_HOLD_FRAMES: usize = 8;
const MAX_FRAMES: usize = 12_000;

pub struct Overflow;

impl Overflow {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Overflow {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Overflow {
    fn name(&self) -> &str {
        "overflow"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        if term.character_count() == 0 {
            return vec![term.render_frame()];
        }

        let mut rng = Rng::new(fnv1a(input));
        let canvas_bottom = term.canvas.bottom;
        let canvas_top = term.canvas.top.max(canvas_bottom);

        let overflow_gradient = Gradient::new(&[hex("f2ebc0"), hex("8dbfb3"), hex("f2ebc0")], 5);
        let overflow_spectrum: Vec<Color> = overflow_gradient.spectrum().to_vec();
        let final_gradient = Gradient::new(&[hex("8A008A"), hex("00D1FF"), hex("FFFFFF")], 12);

        let (min_row, max_row) = term
            .get_characters()
            .iter()
            .fold((i32::MAX, i32::MIN), |(lo, hi), ch| {
                (lo.min(ch.input_coord.row), hi.max(ch.input_coord.row))
            });
        let row_span = f64::from((max_row - min_row).max(1));

        let mut final_color: HashMap<CharacterId, Color> = HashMap::new();
        let mut grouped: BTreeMap<i32, Vec<CharacterId>> = BTreeMap::new();
        for ch in term.get_characters() {
            let progress = f64::from(ch.input_coord.row - min_row) / row_span;
            let color = final_gradient.mapped_color(progress).unwrap_or_else(|| hex("FFFFFF"));
            final_color.insert(ch.id, color);
            grouped.entry(ch.input_coord.row).or_default().push(ch.id);
        }

        // ROW_TOP_TO_BOTTOM, then shuffled like upstream.
        let mut pending: Vec<RowAnim> = grouped
            .into_iter()
            .rev()
            .map(|(final_row, ids)| {
                let cycles = rng.randint(OVERFLOW_CYCLES_LO, OVERFLOW_CYCLES_HI);
                RowAnim {
                    ids,
                    current: rng.randint(canvas_bottom, canvas_top),
                    target: rng.randint(canvas_bottom, canvas_top),
                    final_row,
                    cycles_left: cycles,
                    settled: false,
                }
            })
            .collect();
        rng.shuffle(&mut pending);

        let first_color = overflow_spectrum.first().copied().unwrap_or_else(|| hex("f2ebc0"));
        let starts: HashMap<CharacterId, i32> = pending
            .iter()
            .flat_map(|row| row.ids.iter().copied().map(|id| (id, row.current)))
            .collect();
        for ch in term.get_characters_mut() {
            if let Some(&row) = starts.get(&ch.id) {
                ch.motion.current_coord = Coord {
                    column: ch.input_coord.column,
                    row,
                };
            }
            let symbol = ch.input_symbol.clone();
            ch.animation
                .set_appearance(&symbol, Some(ColorPair::fg(first_color)));
            ch.is_visible = true;
        }

        let mut active: Vec<RowAnim> = Vec::new();
        let mut frames: Vec<String> = Vec::new();
        let mut color_tick = 0usize;

        loop {
            while !pending.is_empty() && active.len() < OVERFLOW_SPEED {
                active.push(pending.remove(0));
            }

            for row in &mut active {
                if row.settled {
                    continue;
                }
                if row.current == row.target {
                    if row.cycles_left > 0 {
                        row.cycles_left -= 1;
                        row.target = rng.randint(canvas_bottom, canvas_top);
                    } else if row.target != row.final_row {
                        row.target = row.final_row;
                    } else {
                        row.settled = true;
                    }
                }
                if !row.settled && row.current != row.target {
                    if row.current < row.target {
                        row.current += 1;
                    } else {
                        row.current -= 1;
                    }
                }
            }

            let mut pos: HashMap<CharacterId, (i32, bool)> = HashMap::new();
            for row in active.iter().chain(pending.iter()) {
                for &id in &row.ids {
                    pos.insert(id, (row.current, row.settled));
                }
            }

            let ov_color = if overflow_spectrum.is_empty() {
                hex("f2ebc0")
            } else {
                overflow_spectrum[(color_tick / COLOR_HOLD) % overflow_spectrum.len()]
            };

            for ch in term.get_characters_mut() {
                if let Some(&(row, settled)) = pos.get(&ch.id) {
                    ch.motion.current_coord = Coord {
                        column: ch.input_coord.column,
                        row,
                    };
                    let color = if settled {
                        final_color.get(&ch.id).copied().unwrap_or(ov_color)
                    } else {
                        ov_color
                    };
                    let symbol = ch.input_symbol.clone();
                    ch.animation
                        .set_appearance(&symbol, Some(ColorPair::fg(color)));
                }
                ch.is_visible = true;
            }

            term.tick();
            frames.push(term.render_frame());
            color_tick = color_tick.saturating_add(1);
            active.retain(|row| !row.settled);

            if (pending.is_empty() && active.is_empty()) || frames.len() >= MAX_FRAMES {
                break;
            }
        }

        for ch in term.get_characters_mut() {
            ch.motion.current_coord = ch.input_coord;
            let color = final_color.get(&ch.id).copied().unwrap_or_else(|| hex("FFFFFF"));
            let symbol = ch.input_symbol.clone();
            ch.animation
                .set_appearance(&symbol, Some(ColorPair::fg(color)));
            ch.is_visible = true;
        }
        term.tick();
        let last = term.render_frame();
        for _ in 0..FINAL_HOLD_FRAMES {
            frames.push(last.clone());
        }
        if frames.is_empty() {
            frames.push(last);
        }
        frames
    }
}

fn hex(s: &str) -> Color {
    Color::from_hex(s).unwrap_or(Color { r: 255, g: 255, b: 255 })
}

fn fnv1a(s: &str) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        self.0
    }

    fn randint(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let span = (i64::from(hi) - i64::from(lo) + 1) as u64;
        lo + (self.next() % span) as i32
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        if items.len() < 2 {
            return;
        }
        for i in (1..items.len()).rev() {
            let j = (self.next() as usize) % (i + 1);
            items.swap(i, j);
        }
    }
}

struct RowAnim {
    ids: Vec<CharacterId>,
    current: i32,
    target: i32,
    final_row: i32,
    cycles_left: i32,
    settled: bool,
}
