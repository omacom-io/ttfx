use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{find_length_of_line, lerp_coord, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

const RAIN_HEX: [&str; 8] = [
    "00315C", "004C8F", "0075DB", "3F91D9", "78B9F2", "9AC8F5", "B8D8F8", "E3EFFC",
];
const FINAL_HEX: [&str; 2] = ["00315C", "E3EFFC"];
const MOVEMENT_SPEED: f64 = 0.15;
const FINAL_GRADIENT_STEPS: usize = 12;
const FINAL_GRADIENT_FRAMES: usize = 5;
const RAINDROP_GRADIENT_STEPS: usize = 8;
const MAX_FRAMES: usize = 20_000;

pub struct Rain;

impl Rain {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Rain {
    fn name(&self) -> &str {
        "rain"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        term.hide_all();
        if term.character_count() == 0 {
            return vec![term.render_frame()];
        }

        let canvas_top = term.canvas.top;
        let rain_palette: Vec<Color> = RAIN_HEX.iter().copied().map(hex_color).collect();
        let final_stops: Vec<Color> = FINAL_HEX.iter().copied().map(hex_color).collect();
        let final_gradient = Gradient::new(&final_stops, FINAL_GRADIENT_STEPS);
        let mut rng = Rng::new(0x5241_494E_3031);

        let mut drops: Vec<Drop> = Vec::new();
        {
            let characters = term.get_characters();
            let text_left = characters
                .iter()
                .map(|c| c.input_coord.column)
                .min()
                .unwrap_or(1);
            let text_right = characters
                .iter()
                .map(|c| c.input_coord.column)
                .max()
                .unwrap_or(1);
            let text_bottom = characters
                .iter()
                .map(|c| c.input_coord.row)
                .min()
                .unwrap_or(1);
            let text_top = characters
                .iter()
                .map(|c| c.input_coord.row)
                .max()
                .unwrap_or(1);
            let col_denom = f64::from((text_right - text_left).max(1));
            let row_denom = f64::from((text_top - text_bottom).max(1));

            for ch in characters {
                let progress = ((f64::from(ch.input_coord.column - text_left) / col_denom)
                    + (f64::from(ch.input_coord.row - text_bottom) / row_denom))
                    / 2.0;
                let final_color = final_gradient
                    .mapped_color(progress)
                    .unwrap_or(rain_palette[0]);
                let rain_color = rain_palette[rng.randint(0, rain_palette.len() - 1)];
                let start = Coord::new(ch.input_coord.column, canvas_top);
                let target = ch.input_coord;
                let fade = {
                    let spectrum =
                        Gradient::new(&[rain_color, final_color], RAINDROP_GRADIENT_STEPS);
                    let colors = spectrum.spectrum().to_vec();
                    if colors.is_empty() {
                        vec![final_color]
                    } else {
                        colors
                    }
                };
                drops.push(Drop {
                    id: ch.id,
                    start,
                    target,
                    speed: MOVEMENT_SPEED * rng.randint(1, 3) as f64,
                    traveled: 0.0,
                    total: find_length_of_line(start, target),
                    rain_color,
                    final_color,
                    fade,
                    phase: Phase::Pending,
                });
            }
        }

        for drop in &drops {
            apply(term.get_character_mut(drop.id), drop.start, drop.rain_color);
        }

        let mut groups: std::collections::BTreeMap<i32, Vec<usize>> =
            std::collections::BTreeMap::new();
        for (idx, drop) in drops.iter().enumerate() {
            groups.entry(drop.target.row).or_default().push(idx);
        }
        let mut rows: Vec<Vec<usize>> = groups.into_iter().map(|(_, row)| row).collect();

        let mut frames = Vec::new();
        loop {
            if let Some(mut row) = rows.pop() {
                let release = rng.randint(1, 3);
                for _ in 0..release {
                    if row.is_empty() {
                        break;
                    }
                    let pick = rng.randint(0, row.len() - 1);
                    let idx = row.remove(pick);
                    term.set_character_visibility(drops[idx].id, true);
                    drops[idx].phase = if drops[idx].total <= 0.0 {
                        Phase::Fading(0)
                    } else {
                        Phase::Falling
                    };
                }
                if !row.is_empty() {
                    rows.push(row);
                }
            }

            for drop in &mut drops {
                match drop.phase {
                    Phase::Falling => {
                        drop.traveled += drop.speed;
                        let t = if drop.total <= 0.0 {
                            1.0
                        } else {
                            (drop.traveled / drop.total).clamp(0.0, 1.0)
                        };
                        let eased = t * t * t * t;
                        let coord = lerp_coord(drop.start, drop.target, eased);
                        apply(term.get_character_mut(drop.id), coord, drop.rain_color);
                        if t >= 1.0 {
                            drop.phase = Phase::Fading(0);
                            apply(
                                term.get_character_mut(drop.id),
                                drop.target,
                                drop.fade.first().copied().unwrap_or(drop.final_color),
                            );
                        }
                    }
                    Phase::Fading(frame) => {
                        let idx = frame / FINAL_GRADIENT_FRAMES;
                        if idx >= drop.fade.len() {
                            drop.phase = Phase::Done;
                            apply(
                                term.get_character_mut(drop.id),
                                drop.target,
                                drop.final_color,
                            );
                        } else {
                            apply(
                                term.get_character_mut(drop.id),
                                drop.target,
                                drop.fade[idx],
                            );
                            drop.phase = Phase::Fading(frame + 1);
                        }
                    }
                    Phase::Pending | Phase::Done => {}
                }
            }

            term.tick();
            frames.push(term.render_frame());

            let finished = rows.is_empty()
                && drops
                    .iter()
                    .all(|d| matches!(d.phase, Phase::Done | Phase::Pending));
            if finished || frames.len() >= MAX_FRAMES {
                break;
            }
        }

        if frames.is_empty() {
            frames.push(term.render_frame());
        }
        frames
    }
}

fn hex_color(hex: &str) -> Color {
    Color::from_hex(hex).unwrap_or(Color::rgb(0, 49, 92))
}

fn apply(ch: Option<&mut crate::engine::character::EffectCharacter>, coord: Coord, color: Color) {
    if let Some(ch) = ch {
        ch.motion.current_coord = coord;
        let symbol = ch.input_symbol.clone();
        ch.animation
            .set_appearance(&symbol, Some(ColorPair::fg(color)));
    }
}

enum Phase {
    Pending,
    Falling,
    Fading(usize),
    Done,
}

struct Drop {
    id: CharacterId,
    start: Coord,
    target: Coord,
    speed: f64,
    traveled: f64,
    total: f64,
    rain_color: Color,
    final_color: Color,
    fade: Vec<Color>,
    phase: Phase,
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
        x as u32
    }

    fn randint(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u32() as usize % (hi - lo + 1))
    }
}
