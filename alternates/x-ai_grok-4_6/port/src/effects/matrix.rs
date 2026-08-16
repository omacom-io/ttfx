use std::collections::HashMap;

use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair, Gradient};

const MATRIX_SYMBOLS: &[&str] = &[
    "ｦ", "ｱ", "ｳ", "ｴ", "ｵ", "ｶ", "ｷ", "ｹ", "ｺ", "ｻ", "ｼ", "ｽ", "ｾ", "ｿ", "ﾀ", "ﾂ",
    "ﾃ", "ﾅ", "ﾆ", "ﾇ", "ﾈ", "ﾊ", "ﾋ", "ﾎ", "ﾏ", "ﾐ", "ﾑ", "ﾒ", "ﾓ", "ﾔ", "ﾕ", "ﾗ",
    "ﾘ", "ﾜ", "0", "1", "2", "3", "4", "5", "7", "8", "9", "Z", ":", ".", "=", "*",
    "+", "-", "<", ">",
];

const RAIN_CYCLES_MIN: i32 = 2;
const RAIN_CYCLES_MAX: i32 = 5;
const START_DELAY_MAX: i32 = 18;
const FALL_DELAY_MAX: i32 = 4;
const INTER_CYCLE_DELAY_MAX: i32 = 8;
const PAUSE_FRAMES: i32 = 6;
const HOLD_FRAMES: i32 = 16;
const SCRAMBLE_FRAMES: i32 = 6;
const RESOLVE_DELAY: i32 = 1;
const MAX_FRAMES: usize = 4000;

pub struct Matrix;

impl Matrix {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Matrix {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Matrix {
    fn name(&self) -> &str {
        "matrix"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        if term.character_count() == 0 {
            return vec![term.render_frame()];
        }
        term.hide_all();

        let highlight = Color::from_hex("f2f230").unwrap_or(Color::rgb(242, 242, 48));
        let rain_gradient = Gradient::new(
            &[
                Color::from_hex("00ff00").unwrap_or(Color::rgb(0, 255, 0)),
                Color::from_hex("15b200").unwrap_or(Color::rgb(21, 178, 0)),
                Color::from_hex("006500").unwrap_or(Color::rgb(0, 101, 0)),
                Color::from_hex("002e00").unwrap_or(Color::rgb(0, 46, 0)),
            ],
            8,
        );

        let snaps: Vec<CharSnap> = term
            .get_characters()
            .iter()
            .map(|ch| CharSnap {
                id: ch.id,
                column: ch.input_coord.column,
                row: ch.input_coord.row,
                symbol: ch.input_symbol.clone(),
                fg: ch.input_fg,
                bg: ch.input_bg,
            })
            .collect();

        let mut by_col: HashMap<i32, Vec<usize>> = HashMap::new();
        for (idx, snap) in snaps.iter().enumerate() {
            by_col.entry(snap.column).or_default().push(idx);
        }

        let top = term.canvas.top;
        let bottom = term.canvas.bottom;
        let height = (top - bottom + 1).max(1);
        let trail = height.clamp(4, 16);

        let mut rng = Rng::new(
            (snaps.len() as u32)
                .wrapping_mul(747796405)
                .wrapping_add(term.canvas.width as u32)
                .wrapping_add(0x4d41_5452),
        );

        let mut columns: Vec<RainColumn> = by_col
            .keys()
            .copied()
            .map(|column| RainColumn {
                column,
                head: top + 1,
                start_delay: rng.range(0, START_DELAY_MAX + 1),
                fall_delay: rng.range(0, FALL_DELAY_MAX + 1),
                fall_wait: 0,
                trail,
                cycles_left: rng.range(RAIN_CYCLES_MIN, RAIN_CYCLES_MAX + 1),
                done: false,
            })
            .collect();
        columns.sort_by_key(|col| col.column);

        let mut resolve_order: Vec<usize> = (0..snaps.len()).collect();
        resolve_order.sort_by(|&a, &b| {
            snaps[b]
                .row
                .cmp(&snaps[a].row)
                .then(snaps[a].column.cmp(&snaps[b].column))
        });

        let mut phase = Phase::Rain;
        let mut pause_left = PAUSE_FRAMES;
        let mut hold_left = HOLD_FRAMES;
        let mut scramble_left: Vec<Option<i32>> = vec![None; snaps.len()];
        let mut resolve_cursor = 0usize;
        let mut resolve_wait = 0i32;
        let resolve_batch = (1 + snaps.len() / 25).max(1);

        let mut frames = Vec::new();
        for _ in 0..MAX_FRAMES {
            match phase {
                Phase::Rain => {
                    hide_unresolved(&mut term, &snaps, &scramble_left);
                    let mut all_done = true;
                    for col in &mut columns {
                        if col.done {
                            continue;
                        }
                        all_done = false;
                        if col.start_delay > 0 {
                            col.start_delay -= 1;
                            continue;
                        }
                        if col.fall_wait > 0 {
                            col.fall_wait -= 1;
                        } else {
                            col.fall_wait = col.fall_delay;
                            col.head -= 1;
                            if col.head + col.trail < bottom {
                                col.cycles_left -= 1;
                                if col.cycles_left <= 0 {
                                    col.done = true;
                                    continue;
                                }
                                col.head = top + 1;
                                col.start_delay = rng.range(0, INTER_CYCLE_DELAY_MAX + 1);
                                continue;
                            }
                        }
                        paint_column(
                            &mut term,
                            col,
                            &snaps,
                            &by_col,
                            &rain_gradient,
                            highlight,
                            &mut rng,
                        );
                    }
                    if all_done {
                        phase = Phase::Pause;
                    }
                }
                Phase::Pause => {
                    hide_unresolved(&mut term, &snaps, &scramble_left);
                    pause_left -= 1;
                    if pause_left <= 0 {
                        phase = Phase::Resolve;
                    }
                }
                Phase::Resolve => {
                    if resolve_wait > 0 {
                        resolve_wait -= 1;
                    } else {
                        for _ in 0..resolve_batch {
                            if resolve_cursor < resolve_order.len() {
                                let idx = resolve_order[resolve_cursor];
                                scramble_left[idx] = Some(SCRAMBLE_FRAMES);
                                resolve_cursor += 1;
                            }
                        }
                        resolve_wait = RESOLVE_DELAY;
                    }

                    for (idx, remaining) in scramble_left.iter_mut().enumerate() {
                        let Some(left) = remaining.as_mut() else {
                            continue;
                        };
                        let snap = &snaps[idx];
                        if *left <= 1 {
                            apply_original(&mut term, snap);
                            *left = 0;
                        } else {
                            *left -= 1;
                            let symbol = *rng.choice(MATRIX_SYMBOLS);
                            let color = if *left % 2 == 0 {
                                highlight
                            } else {
                                rain_gradient
                                    .mapped_color(0.15)
                                    .unwrap_or(Color::rgb(0, 255, 0))
                            };
                            if let Some(ch) = term.get_character_mut(snap.id) {
                                ch.animation
                                    .set_appearance(symbol, Some(ColorPair::fg(color)));
                                ch.is_visible = true;
                            }
                        }
                    }

                    let finished = resolve_cursor >= resolve_order.len()
                        && scramble_left.iter().all(|state| matches!(state, Some(0)));
                    if finished {
                        phase = Phase::Hold;
                    }
                }
                Phase::Hold => {
                    hold_left -= 1;
                    if hold_left <= 0 {
                        frames.push(term.render_frame());
                        break;
                    }
                }
            }

            frames.push(term.render_frame());
        }

        if frames.is_empty() {
            for snap in &snaps {
                apply_original(&mut term, snap);
            }
            frames.push(term.render_frame());
        }
        frames
    }
}

struct CharSnap {
    id: CharacterId,
    column: i32,
    row: i32,
    symbol: String,
    fg: Option<Color>,
    bg: Option<Color>,
}

struct RainColumn {
    column: i32,
    head: i32,
    start_delay: i32,
    fall_delay: i32,
    fall_wait: i32,
    trail: i32,
    cycles_left: i32,
    done: bool,
}

enum Phase {
    Rain,
    Pause,
    Resolve,
    Hold,
}

struct Rng {
    state: u32,
}

impl Rng {
    fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }

    fn range(&mut self, min: i32, max_exclusive: i32) -> i32 {
        if max_exclusive <= min {
            return min;
        }
        let span = (max_exclusive - min) as u32;
        min + (self.next_u32() % span) as i32
    }

    fn choice<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.next_u32() as usize % items.len()]
    }
}

fn hide_unresolved(term: &mut Terminal, snaps: &[CharSnap], scramble_left: &[Option<i32>]) {
    for (idx, snap) in snaps.iter().enumerate() {
        if !matches!(scramble_left.get(idx), Some(Some(0))) {
            term.set_character_visibility(snap.id, false);
        }
    }
}

fn paint_column(
    term: &mut Terminal,
    col: &RainColumn,
    snaps: &[CharSnap],
    by_col: &HashMap<i32, Vec<usize>>,
    rain_gradient: &Gradient,
    highlight: Color,
    rng: &mut Rng,
) {
    let Some(indices) = by_col.get(&col.column) else {
        return;
    };
    for &idx in indices {
        let snap = &snaps[idx];
        let dist = snap.row - col.head;
        if dist < 0 || dist >= col.trail {
            continue;
        }
        let color = if dist == 0 {
            highlight
        } else {
            let t = if col.trail <= 1 {
                0.0
            } else {
                dist as f64 / f64::from(col.trail - 1)
            };
            rain_gradient
                .mapped_color(t.clamp(0.0, 1.0))
                .unwrap_or(Color::rgb(0, 80, 0))
        };
        let symbol = *rng.choice(MATRIX_SYMBOLS);
        if let Some(ch) = term.get_character_mut(snap.id) {
            ch.animation
                .set_appearance(symbol, Some(ColorPair::fg(color)));
            ch.is_visible = true;
        }
    }
}

fn apply_original(term: &mut Terminal, snap: &CharSnap) {
    let colors = if snap.fg.is_some() || snap.bg.is_some() {
        Some(ColorPair::new(snap.fg, snap.bg))
    } else {
        None
    };
    if let Some(ch) = term.get_character_mut(snap.id) {
        ch.animation.set_appearance(&snap.symbol, colors);
        ch.is_visible = true;
    }
}
