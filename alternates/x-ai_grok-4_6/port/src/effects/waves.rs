use std::collections::BTreeMap;

use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair, Gradient};

const WAVE_SYMBOLS: [&str; 15] = [
    "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃", "▂", "▁",
];
const WAVE_COUNT: usize = 7;
const WAVE_LENGTH: usize = 2;
const FINAL_GRADIENT_STEPS: usize = 12;
const FINAL_GRADIENT_FRAMES: usize = 5;
const MAX_FRAMES: usize = 100_000;

pub struct Waves;

impl Waves {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Waves {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Waves {
    fn name(&self) -> &str {
        "waves"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_input(input, TerminalConfig::default());
        if terminal.character_count() == 0 {
            return vec![terminal.render_frame()];
        }

        let wave_gradient = Gradient::new(
            &[
                Color::rgb(0x8a, 0x00, 0x8a),
                Color::rgb(0x00, 0xd1, 0xff),
                Color::rgb(0xff, 0xff, 0xff),
            ],
            6,
        );
        let final_gradient = Gradient::new(
            &[
                Color::rgb(0xab, 0x48, 0xff),
                Color::rgb(0xe7, 0xb2, 0xb2),
                Color::rgb(0xff, 0xfe, 0xbd),
            ],
            FINAL_GRADIENT_STEPS,
        );
        let wave_end = wave_gradient
            .spectrum()
            .last()
            .copied()
            .unwrap_or(Color::rgb(0xff, 0xff, 0xff));

        let (min_row, max_row) = {
            let chars = terminal.get_characters();
            let min_row = chars.iter().map(|c| c.input_coord.row).min().unwrap_or(1);
            let max_row = chars.iter().map(|c| c.input_coord.row).max().unwrap_or(1);
            (min_row, max_row)
        };

        let mut columns: BTreeMap<i32, Vec<CharacterId>> = BTreeMap::new();
        let mut states: Vec<CharState> = Vec::new();

        for ch in terminal.get_characters() {
            if ch.input_symbol == " " && ch.input_fg.is_none() && ch.input_bg.is_none() {
                continue;
            }
            let progress = if max_row == min_row {
                0.0
            } else {
                f64::from(ch.input_coord.row - min_row) / f64::from(max_row - min_row)
            };
            let final_color = final_gradient
                .mapped_color(progress)
                .or_else(|| final_gradient.get(0))
                .unwrap_or(Color::rgb(0xff, 0xff, 0xff));
            columns.entry(ch.input_coord.column).or_default().push(ch.id);
            states.push(CharState {
                id: ch.id,
                phase: Phase::Pending,
                final_color,
            });
        }

        if states.is_empty() {
            terminal.show_all();
            return vec![terminal.render_frame()];
        }

        let mut pending: Vec<Vec<CharacterId>> = columns.into_values().collect();
        let mut frames = Vec::new();

        loop {
            if let Some(group) = pending.first().cloned() {
                pending.remove(0);
                for id in group {
                    if let Some(state) = states.iter_mut().find(|s| s.id == id) {
                        state.phase = Phase::Wave {
                            step: 0,
                            hold: WAVE_LENGTH,
                        };
                    }
                    terminal.set_character_visibility(id, true);
                    paint_wave(&mut terminal, id, 0, &wave_gradient);
                }
            }

            frames.push(terminal.render_frame());
            terminal.tick();

            let mut alive = !pending.is_empty();
            for i in 0..states.len() {
                if advance_state(i, &mut states, &mut terminal, &wave_gradient, wave_end) {
                    alive = true;
                }
            }

            if !alive {
                frames.push(terminal.render_frame());
                break;
            }
            if frames.len() >= MAX_FRAMES {
                break;
            }
        }

        frames
    }
}

#[derive(Clone, Copy)]
enum Phase {
    Pending,
    Wave { step: usize, hold: usize },
    Settle { step: usize, hold: usize },
    Done,
}

struct CharState {
    id: CharacterId,
    phase: Phase,
    final_color: Color,
}

fn advance_state(
    index: usize,
    states: &mut [CharState],
    terminal: &mut Terminal,
    wave_gradient: &Gradient,
    wave_end: Color,
) -> bool {
    let id = states[index].id;
    let final_color = states[index].final_color;
    let total_wave_steps = WAVE_COUNT * WAVE_SYMBOLS.len();

    match states[index].phase {
        Phase::Pending => false,
        Phase::Done => false,
        Phase::Wave { step, hold } => {
            if hold > 1 {
                states[index].phase = Phase::Wave {
                    step,
                    hold: hold - 1,
                };
                return true;
            }
            let next = step + 1;
            if next >= total_wave_steps {
                states[index].phase = Phase::Settle {
                    step: 0,
                    hold: FINAL_GRADIENT_FRAMES,
                };
                paint_settle(terminal, id, 0, wave_end, final_color);
            } else {
                states[index].phase = Phase::Wave {
                    step: next,
                    hold: WAVE_LENGTH,
                };
                paint_wave(terminal, id, next, wave_gradient);
            }
            true
        }
        Phase::Settle { step, hold } => {
            if hold > 1 {
                states[index].phase = Phase::Settle {
                    step,
                    hold: hold - 1,
                };
                return true;
            }
            let next = step + 1;
            let settle_len = settle_len();
            if next >= settle_len {
                states[index].phase = Phase::Done;
                paint_final(terminal, id, final_color);
                false
            } else {
                states[index].phase = Phase::Settle {
                    step: next,
                    hold: FINAL_GRADIENT_FRAMES,
                };
                paint_settle(terminal, id, next, wave_end, final_color);
                true
            }
        }
    }
}

fn settle_len() -> usize {
    Gradient::new(&[Color::rgb(0, 0, 0), Color::rgb(0, 0, 0)], FINAL_GRADIENT_STEPS).len()
}

fn paint_wave(terminal: &mut Terminal, id: CharacterId, step: usize, wave_gradient: &Gradient) {
    let idx = step % WAVE_SYMBOLS.len();
    let symbol = WAVE_SYMBOLS[idx];
    let progress = if WAVE_SYMBOLS.len() <= 1 {
        0.0
    } else {
        idx as f64 / (WAVE_SYMBOLS.len() - 1) as f64
    };
    let color = wave_gradient
        .mapped_color(progress)
        .or_else(|| wave_gradient.get(0))
        .unwrap_or(Color::rgb(0xff, 0xff, 0xff));
    if let Some(ch) = terminal.get_character_mut(id) {
        ch.animation
            .set_appearance(symbol, Some(ColorPair::fg(color)));
    }
}

fn paint_settle(
    terminal: &mut Terminal,
    id: CharacterId,
    step: usize,
    wave_end: Color,
    final_color: Color,
) {
    let gradient = Gradient::new(&[wave_end, final_color], FINAL_GRADIENT_STEPS);
    let color = gradient
        .get(step.min(gradient.len().saturating_sub(1)))
        .unwrap_or(final_color);
    paint_final(terminal, id, color);
}

fn paint_final(terminal: &mut Terminal, id: CharacterId, color: Color) {
    if let Some(ch) = terminal.get_character_mut(id) {
        let symbol = ch.input_symbol.clone();
        ch.animation
            .set_appearance(&symbol, Some(ColorPair::fg(color)));
    }
}
