//! Some characters are swapped into the wrong position and slowly corrected.

use std::collections::HashSet;

use super::Effect;
use crate::engine::{CharacterId, Terminal, TerminalConfig};
use crate::utils::geometry::{find_length_of_line, lerp_coord, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

const ERROR_PAIRS: f64 = 0.1;
const SWAP_DELAY: u32 = 10;
const MOVEMENT_SPEED: f64 = 0.5;
const FINAL_GRADIENT_STEPS: usize = 12;
const CORRECT_GRADIENT_STEPS: usize = 10;
const FINAL_GRADIENT_FRAMES: usize = 5;
const MAX_FRAMES: usize = 100_000;

fn hex_color(hex: &str) -> Color {
    Color::from_hex(hex).unwrap_or(Color::rgb(255, 255, 255))
}

fn error_color() -> Color {
    hex_color("e74c3c")
}

fn correct_color() -> Color {
    hex_color("45bf55")
}

fn final_stops() -> [Color; 3] {
    [hex_color("8A008A"), hex_color("00D1FF"), hex_color("FFFFFF")]
}

/// Python `easing.in_out_quad`.
fn in_out_quad(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        2.0 * t * t
    } else {
        let u = -2.0 * t + 2.0;
        1.0 - (u * u) / 2.0
    }
}

fn is_content(symbol: &str, fg: Option<Color>, bg: Option<Color>) -> bool {
    symbol != " " && symbol != "\t" || fg.is_some() || bg.is_some()
}

fn paint(term: &mut Terminal, id: CharacterId, color: Color) {
    if let Some(ch) = term.get_character_mut(id) {
        let symbol = ch.input_symbol.clone();
        ch.animation.set_appearance(&symbol, Some(ColorPair::fg(color)));
        ch.is_visible = true;
    }
}

fn fnv1a(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.0 >> 32) as u32
    }

    fn gen_index(&mut self, max: usize) -> usize {
        if max == 0 {
            0
        } else {
            (self.next_u32() as usize) % max
        }
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.gen_index(i + 1);
            items.swap(i, j);
        }
    }
}

struct Mover {
    id: CharacterId,
    start: Coord,
    end: Coord,
    distance: f64,
    progress: f64,
}

impl Mover {
    fn new(id: CharacterId, start: Coord, end: Coord) -> Self {
        Self {
            id,
            start,
            end,
            distance: find_length_of_line(start, end),
            progress: 0.0,
        }
    }

    /// Advance along the path. Returns `true` while still travelling.
    fn tick(&mut self) -> bool {
        if self.distance <= f64::EPSILON {
            self.progress = 1.0;
            return false;
        }
        self.progress = (self.progress + MOVEMENT_SPEED / self.distance).min(1.0);
        self.progress < 1.0
    }

    fn coord(&self) -> Coord {
        if self.progress >= 1.0 || self.distance <= f64::EPSILON {
            self.end
        } else {
            lerp_coord(self.start, self.end, in_out_quad(self.progress))
        }
    }
}

struct Fade {
    id: CharacterId,
    spectrum: Vec<Color>,
    step: usize,
    hold: usize,
}

impl Fade {
    fn new(id: CharacterId, spectrum: Vec<Color>) -> Self {
        Self {
            id,
            spectrum,
            step: 0,
            hold: 0,
        }
    }

    fn color(&self) -> Option<Color> {
        self.spectrum.get(self.step).copied()
    }

    /// Hold each gradient step for `FINAL_GRADIENT_FRAMES`. `false` once finished.
    fn advance(&mut self) -> bool {
        if self.spectrum.is_empty() {
            return false;
        }
        self.hold += 1;
        if self.hold < FINAL_GRADIENT_FRAMES {
            return true;
        }
        self.hold = 0;
        if self.step + 1 < self.spectrum.len() {
            self.step += 1;
            true
        } else {
            false
        }
    }
}

fn text_bounds(term: &Terminal) -> Option<(i32, i32, i32, i32)> {
    let mut iter = term
        .get_characters()
        .iter()
        .filter(|ch| is_content(&ch.input_symbol, ch.input_fg, ch.input_bg))
        .map(|ch| ch.input_coord);
    let first = iter.next()?;
    let mut min_col = first.column;
    let mut max_col = first.column;
    let mut min_row = first.row;
    let mut max_row = first.row;
    for coord in iter {
        min_col = min_col.min(coord.column);
        max_col = max_col.max(coord.column);
        min_row = min_row.min(coord.row);
        max_row = max_row.max(coord.row);
    }
    Some((min_col, max_col, min_row, max_row))
}

fn final_color_map(term: &Terminal) -> Vec<(CharacterId, Color)> {
    let gradient = Gradient::new(&final_stops(), FINAL_GRADIENT_STEPS);
    let fallback = gradient.get(0).unwrap_or(Color::rgb(255, 255, 255));
    let (min_row, max_row) = match text_bounds(term) {
        Some((_, _, min_row, max_row)) => (min_row, max_row),
        None => return Vec::new(),
    };
    let span = f64::from(max_row - min_row);
    term.get_characters()
        .iter()
        .filter(|ch| is_content(&ch.input_symbol, ch.input_fg, ch.input_bg))
        .map(|ch| {
            let progress = if span == 0.0 {
                0.0
            } else {
                f64::from(ch.input_coord.row - min_row) / span
            };
            let color = gradient.mapped_color(progress).unwrap_or(fallback);
            (ch.id, color)
        })
        .collect()
}

fn correct_spectrum(final_color: Color) -> Vec<Color> {
    let gradient = Gradient::new(&[correct_color(), final_color], CORRECT_GRADIENT_STEPS);
    let mut spectrum = gradient.spectrum().to_vec();
    if spectrum.is_empty() {
        spectrum.push(final_color);
    }
    spectrum
}

/// Some characters are swapped and slowly slide back to the correct position.
#[derive(Clone, Debug, Default)]
pub struct Errorcorrect;

impl Errorcorrect {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Errorcorrect {
    fn name(&self) -> &str {
        "errorcorrect"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        let finals = final_color_map(&term);
        if finals.is_empty() {
            return vec![term.render_frame()];
        }

        let mut ids: Vec<CharacterId> = finals.iter().map(|(id, _)| *id).collect();
        let mut rng = Rng::new(fnv1a(input));
        rng.shuffle(&mut ids);

        let mut pair_count = ((ids.len() as f64) * ERROR_PAIRS) as usize;
        if pair_count % 2 == 1 {
            pair_count -= 1;
        }
        pair_count = pair_count.min(ids.len());

        let mut pending: Vec<(CharacterId, CharacterId)> = Vec::new();
        let mut paired: HashSet<CharacterId> = HashSet::new();
        let mut i = 0;
        while i + 1 < pair_count {
            let a = ids[i];
            let b = ids[i + 1];
            pending.push((a, b));
            paired.insert(a);
            paired.insert(b);
            i += 2;
        }

        for &(id_a, id_b) in &pending {
            let home_a = term.get_character(id_a).map(|ch| ch.input_coord);
            let home_b = term.get_character(id_b).map(|ch| ch.input_coord);
            if let (Some(home_a), Some(home_b)) = (home_a, home_b) {
                if let Some(ch) = term.get_character_mut(id_a) {
                    ch.motion.current_coord = home_b;
                }
                if let Some(ch) = term.get_character_mut(id_b) {
                    ch.motion.current_coord = home_a;
                }
            }
            paint(&mut term, id_a, error_color());
            paint(&mut term, id_b, error_color());
        }

        let mut fades: Vec<Fade> = Vec::new();
        for &(id, final_color) in &finals {
            if paired.contains(&id) {
                continue;
            }
            let fade = Fade::new(id, correct_spectrum(final_color));
            if let Some(color) = fade.color() {
                paint(&mut term, id, color);
            }
            fades.push(fade);
        }

        let final_of = |id: CharacterId| -> Color {
            finals
                .iter()
                .find(|(fid, _)| *fid == id)
                .map(|(_, c)| *c)
                .unwrap_or_else(correct_color)
        };

        let mut movers: Vec<Mover> = Vec::new();
        let mut swap_delay: u32 = 0;
        let mut frames: Vec<String> = Vec::new();

        loop {
            if !pending.is_empty() {
                if swap_delay == 0 {
                    let (id_a, id_b) = pending.remove(0);
                    for id in [id_a, id_b] {
                        if let Some(ch) = term.get_character(id) {
                            let mover = Mover::new(id, ch.current_coord(), ch.input_coord);
                            paint(&mut term, id, error_color());
                            movers.push(mover);
                        }
                    }
                    swap_delay = SWAP_DELAY;
                } else {
                    swap_delay -= 1;
                }
            }

            let mut still_moving = Vec::new();
            for mut mover in movers.drain(..) {
                let travelling = mover.tick();
                let coord = mover.coord();
                if let Some(ch) = term.get_character_mut(mover.id) {
                    ch.motion.current_coord = coord;
                }
                if travelling {
                    still_moving.push(mover);
                } else {
                    if let Some(ch) = term.get_character_mut(mover.id) {
                        ch.motion.current_coord = mover.end;
                    }
                    let fade = Fade::new(mover.id, correct_spectrum(final_of(mover.id)));
                    if let Some(color) = fade.color() {
                        paint(&mut term, mover.id, color);
                    }
                    fades.push(fade);
                }
            }
            movers = still_moving;

            for fade in &fades {
                if let Some(color) = fade.color() {
                    paint(&mut term, fade.id, color);
                }
            }

            frames.push(term.render_frame());

            fades.retain_mut(|fade| fade.advance());

            if pending.is_empty() && movers.is_empty() && fades.is_empty() {
                break;
            }
            if frames.len() >= MAX_FRAMES {
                break;
            }
        }

        if frames.is_empty() {
            frames.push(term.render_frame());
        }
        frames
    }
}
