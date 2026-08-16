//! Smoke drifts across the canvas to conceal, then reveal, the input text.
//!
//! Port of `terminaltexteffects/effects/effect_smoke.py`. Characters are
//! unveiled in shuffled order cycling through the default smoke greys, fade
//! out (dissipate), then fade back in along the default final gradient.

use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair, Gradient};

const SMOKE_HEX: &[&str] = &["818596", "73747a", "5d5f6b", "3c3d43", "2a2c38"];
const FINAL_HEX: &[&str] = &["8A008A", "00D1FF", "FFFFFF"];
const SMOKE_STEPS: usize = 5;
const SMOKE_HOLD: u32 = 10;
const DISSIPATE_STEPS: usize = 6;
const DISSIPATE_HOLD: u32 = 5;
const FINAL_STEPS: usize = 12;
const FINAL_HOLD: u32 = 5;
const LAUNCH_PER_FRAME: usize = 3;
const MAX_FRAMES: usize = 2500;

#[derive(Clone, Debug, Default)]
pub struct Smoke;

impl Smoke {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Smoke {
    fn name(&self) -> &str {
        "smoke"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        term.hide_all();

        let n = term.character_count();
        if n == 0 {
            return vec![term.render_frame()];
        }

        let smoke_stops: Vec<Color> = SMOKE_HEX.iter().filter_map(|h| Color::from_hex(h)).collect();
        let final_stops: Vec<Color> = FINAL_HEX.iter().filter_map(|h| Color::from_hex(h)).collect();
        let fallback = Color::rgb(0x80, 0x80, 0x80);
        let black = Color::rgb(0, 0, 0);

        let smoke_spec: Vec<Color> = {
            let g = Gradient::new(&smoke_stops, SMOKE_STEPS);
            if g.is_empty() {
                vec![fallback]
            } else {
                g.spectrum().to_vec()
            }
        };
        let final_grad = Gradient::new(&final_stops, FINAL_STEPS);

        let snapshot: Vec<(CharacterId, String, i32)> = term
            .get_characters()
            .iter()
            .map(|ch| (ch.id, ch.input_symbol.clone(), ch.input_coord.row))
            .collect();

        let min_row = snapshot.iter().map(|s| s.2).min().unwrap_or(1);
        let max_row = snapshot.iter().map(|s| s.2).max().unwrap_or(1);
        let row_span = (max_row - min_row).max(1) as f64;

        let ids: Vec<CharacterId> = snapshot.iter().map(|s| s.0).collect();
        let symbols: Vec<String> = snapshot.iter().map(|s| s.1.clone()).collect();
        let final_colors: Vec<Color> = snapshot
            .iter()
            .map(|s| {
                let progress = (s.2 - min_row) as f64 / row_span;
                final_grad.mapped_color(progress).unwrap_or(fallback)
            })
            .collect();

        let dissipate_specs: Vec<Vec<Color>> = final_colors
            .iter()
            .map(|&fc| {
                let g = Gradient::new(&[fc, black], DISSIPATE_STEPS);
                g.spectrum().iter().skip(1).copied().collect()
            })
            .collect();
        let reveal_specs: Vec<Vec<Color>> = final_colors
            .iter()
            .map(|&fc| Gradient::new(&[black, fc], FINAL_STEPS).spectrum().to_vec())
            .collect();

        let mut order: Vec<usize> = (0..n).collect();
        lcg_shuffle(&mut order, 0xC0FF_EE54_u64.wrapping_mul(n as u64 + 1));

        let smoke_len = (smoke_spec.len() as u32).saturating_mul(SMOKE_HOLD);
        let dissipate_color_frames = dissipate_specs
            .iter()
            .map(|s| s.len() as u32)
            .max()
            .unwrap_or(0)
            .saturating_mul(DISSIPATE_HOLD);
        let dissipate_len = dissipate_color_frames.saturating_add(1);
        let reveal_len = reveal_specs
            .iter()
            .map(|s| s.len() as u32)
            .max()
            .unwrap_or(1)
            .saturating_mul(FINAL_HOLD)
            .max(1);

        let mut smoke_age: Vec<Option<u32>> = vec![None; n];
        let mut launched = 0usize;
        let mut phase = Phase::Smoke;
        let mut phase_frame: u32 = 0;
        let mut frames = Vec::new();

        loop {
            if frames.len() >= MAX_FRAMES {
                break;
            }
            match phase {
                Phase::Smoke => {
                    for _ in 0..LAUNCH_PER_FRAME {
                        if launched >= n {
                            break;
                        }
                        let i = order[launched];
                        smoke_age[i] = Some(0);
                        term.set_character_visibility(ids[i], true);
                        launched += 1;
                    }

                    let mut any_playing = false;
                    for i in 0..n {
                        let Some(age) = smoke_age[i] else {
                            continue;
                        };
                        let idx = if smoke_len == 0 {
                            0
                        } else {
                            ((age / SMOKE_HOLD) as usize).min(smoke_spec.len() - 1)
                        };
                        paint(&mut term, ids[i], &symbols[i], smoke_spec[idx], true);
                        if age + 1 < smoke_len {
                            smoke_age[i] = Some(age + 1);
                            any_playing = true;
                        } else {
                            smoke_age[i] = Some(smoke_len);
                        }
                    }

                    if launched >= n && !any_playing {
                        phase = Phase::Dissipate;
                        phase_frame = 0;
                    }
                }
                Phase::Dissipate => {
                    if phase_frame >= dissipate_len {
                        phase = Phase::Reveal;
                        phase_frame = 0;
                    } else {
                        for i in 0..n {
                            let spec = &dissipate_specs[i];
                            if spec.is_empty() || phase_frame >= dissipate_color_frames {
                                paint(&mut term, ids[i], " ", black, true);
                            } else {
                                let idx = ((phase_frame / DISSIPATE_HOLD) as usize).min(spec.len() - 1);
                                paint(&mut term, ids[i], &symbols[i], spec[idx], true);
                            }
                        }
                        phase_frame = phase_frame.saturating_add(1);
                    }
                }
                Phase::Reveal => {
                    if phase_frame >= reveal_len {
                        phase = Phase::Done;
                    } else {
                        for i in 0..n {
                            let spec = &reveal_specs[i];
                            let color = if spec.is_empty() {
                                final_colors[i]
                            } else {
                                let idx = ((phase_frame / FINAL_HOLD) as usize).min(spec.len() - 1);
                                spec[idx]
                            };
                            paint(&mut term, ids[i], &symbols[i], color, true);
                        }
                        phase_frame = phase_frame.saturating_add(1);
                    }
                }
                Phase::Done => break,
            }
            frames.push(term.render_frame());
            if matches!(phase, Phase::Done) {
                break;
            }
        }

        if frames.is_empty() {
            term.show_all();
            frames.push(term.render_frame());
        }
        frames
    }
}

#[derive(Clone, Copy)]
enum Phase {
    Smoke,
    Dissipate,
    Reveal,
    Done,
}

fn paint(term: &mut Terminal, id: CharacterId, symbol: &str, color: Color, visible: bool) {
    term.set_character_visibility(id, visible);
    if let Some(ch) = term.get_character_mut(id) {
        ch.animation.set_appearance(symbol, Some(ColorPair::fg(color)));
    }
}

fn lcg_shuffle<T>(items: &mut [T], mut state: u64) {
    for i in (1..items.len()).rev() {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let j = (state as usize) % (i + 1);
        items.swap(i, j);
    }
}
