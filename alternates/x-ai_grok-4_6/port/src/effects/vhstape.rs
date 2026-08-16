use std::collections::HashMap;

use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

const TOTAL_GLITCH_TIME: u32 = 1000;
const GLITCH_LINE_CHANCE: f64 = 0.05;
const NOISE_CHANCE: f64 = 0.004;
const WAVE_START_CHANCE: f64 = 0.02;
const WAVE_WIDTH: isize = 3;

pub struct Vhstape;

impl Vhstape {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Vhstape {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Vhstape {
    fn name(&self) -> &str {
        "vhstape"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        if term.character_count() == 0 {
            return Vec::new();
        }

        let glitch_line_colors = [
            hex("ffffff"),
            hex("ff0000"),
            hex("00ff00"),
            hex("0000ff"),
            hex("ffffff"),
        ];
        let glitch_wave_colors = [
            hex("ffffff"),
            hex("ff0000"),
            hex("00ff00"),
            hex("0000ff"),
            hex("ffffff"),
        ];
        let noise_colors = [
            hex("1e1e1f"),
            hex("3c3b3d"),
            hex("6d6c70"),
            hex("a2a1a6"),
            hex("cbc9cf"),
            hex("ffffff"),
        ];
        let gradient = Gradient::new(
            &[hex("ab48ff"), hex("e7b2b2"), hex("aaf5d0")],
            12,
        );
        let final_colors = build_final_colors(&term, &gradient);

        let mut row_map: HashMap<i32, Vec<CharacterId>> = HashMap::new();
        for ch in term.get_characters() {
            row_map.entry(ch.input_coord.row).or_default().push(ch.id);
        }
        let mut rows: Vec<i32> = row_map.keys().copied().collect();
        rows.sort_by(|a, b| b.cmp(a));
        let mut lines: Vec<Line> = rows
            .into_iter()
            .filter_map(|row| row_map.remove(&row))
            .map(|ids| Line {
                ids,
                glitching: false,
                hold: 0,
            })
            .collect();

        let mut rng = Rng::from_input(input);
        term.show_all();

        let mut frames = Vec::with_capacity(TOTAL_GLITCH_TIME as usize + 8);
        let mut wave: Option<Wave> = None;

        for _ in 0..TOTAL_GLITCH_TIME {
            if let Some(active) = wave.as_mut() {
                let back = active.head - active.dir * WAVE_WIDTH;
                if back >= 0 {
                    if let Some(line) = lines.get_mut(back as usize) {
                        restore_line(&mut term, line, &final_colors);
                    }
                }
                if active.head >= 0 && (active.head as usize) < lines.len() {
                    let idx = active.head as usize;
                    let offset = active.offset;
                    glitch_line(
                        &mut term,
                        &mut lines[idx],
                        &glitch_wave_colors,
                        offset,
                        &mut rng,
                    );
                    active.head += active.dir;
                } else {
                    for line in lines.iter_mut() {
                        if line.glitching {
                            restore_line(&mut term, line, &final_colors);
                        }
                    }
                    wave = None;
                }
            } else {
                for line in lines.iter_mut() {
                    if line.glitching {
                        if line.hold > 0 {
                            line.hold -= 1;
                        } else {
                            restore_line(&mut term, line, &final_colors);
                        }
                    } else if rng.chance(GLITCH_LINE_CHANCE) {
                        let offset = rng.gen_range(1, 9) * rng.sign();
                        glitch_line(
                            &mut term,
                            line,
                            &glitch_line_colors,
                            offset,
                            &mut rng,
                        );
                        line.hold = rng.gen_range(5, 16) as u32;
                    }
                }
                if rng.chance(WAVE_START_CHANCE) && !lines.is_empty() {
                    for line in lines.iter_mut() {
                        if line.glitching {
                            restore_line(&mut term, line, &final_colors);
                        }
                    }
                    let dir = rng.sign() as isize;
                    let head = if dir > 0 {
                        0
                    } else {
                        lines.len() as isize - 1
                    };
                    wave = Some(Wave {
                        head,
                        dir,
                        offset: rng.gen_range(4, 16) * rng.sign(),
                    });
                }
            }

            if rng.chance(NOISE_CHANCE) {
                for line in &lines {
                    snow_line(&mut term, line, &noise_colors, &mut rng);
                }
            }

            term.tick();
            frames.push(term.render_frame());
        }

        for line in lines.iter_mut() {
            restore_line(&mut term, line, &final_colors);
        }
        for _ in 0..6 {
            term.tick();
            frames.push(term.render_frame());
        }
        frames
    }
}

struct Line {
    ids: Vec<CharacterId>,
    glitching: bool,
    hold: u32,
}

struct Wave {
    head: isize,
    dir: isize,
    offset: i32,
}

fn hex(value: &str) -> Color {
    Color::from_hex(value).unwrap_or(Color::rgb(255, 255, 255))
}

fn build_final_colors(term: &Terminal, gradient: &Gradient) -> HashMap<CharacterId, Color> {
    let chars = term.get_characters();
    let min_row = chars.iter().map(|ch| ch.input_coord.row).min().unwrap_or(1);
    let max_row = chars.iter().map(|ch| ch.input_coord.row).max().unwrap_or(1);
    let span = f64::from((max_row - min_row).max(1));
    chars
        .iter()
        .map(|ch| {
            let progress = f64::from(ch.input_coord.row - min_row) / span;
            let color = gradient
                .mapped_color(progress)
                .unwrap_or(Color::rgb(255, 255, 255));
            (ch.id, color)
        })
        .collect()
}

fn glitch_line(
    term: &mut Terminal,
    line: &mut Line,
    colors: &[Color],
    offset: i32,
    rng: &mut Rng,
) {
    for id in &line.ids {
        if let Some(ch) = term.get_character_mut(*id) {
            ch.motion.current_coord = Coord::new(
                ch.input_coord.column + offset,
                ch.input_coord.row,
            );
            let symbol = ch.input_symbol.clone();
            let color = *rng.choice(colors);
            ch.animation
                .set_appearance(&symbol, Some(ColorPair::fg(color)));
        }
    }
    line.glitching = true;
}

fn restore_line(
    term: &mut Terminal,
    line: &mut Line,
    final_colors: &HashMap<CharacterId, Color>,
) {
    for id in &line.ids {
        if let Some(ch) = term.get_character_mut(*id) {
            ch.motion.current_coord = ch.input_coord;
            let symbol = ch.input_symbol.clone();
            let color = final_colors
                .get(id)
                .copied()
                .unwrap_or(Color::rgb(255, 255, 255));
            ch.animation
                .set_appearance(&symbol, Some(ColorPair::fg(color)));
        }
    }
    line.glitching = false;
    line.hold = 0;
}

fn snow_line(term: &mut Terminal, line: &Line, colors: &[Color], rng: &mut Rng) {
    for id in &line.ids {
        if let Some(ch) = term.get_character_mut(*id) {
            let symbol = ch.animation.current_character_visual.symbol.clone();
            let color = *rng.choice(colors);
            ch.animation
                .set_appearance(&symbol, Some(ColorPair::fg(color)));
        }
    }
}

struct Rng {
    state: u64,
}

impl Rng {
    fn from_input(input: &str) -> Self {
        let mut state = 0x9e37_79b9_7f4a_7c15u64;
        for byte in input.as_bytes() {
            state = state
                .wrapping_mul(0x0000_0100_0000_01b3)
                .wrapping_add(u64::from(*byte));
        }
        Self { state: state | 1 }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn gen_range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u32() as i32).rem_euclid(hi - lo)
    }

    fn chance(&mut self, p: f64) -> bool {
        (self.next_u32() as f64) / 4294967295.0 < p
    }

    fn sign(&mut self) -> i32 {
        if self.next_u32() & 1 == 0 {
            1
        } else {
            -1
        }
    }

    fn choice<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.next_u32() as usize % items.len()]
    }
}
