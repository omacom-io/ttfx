use std::collections::HashMap;

use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{
    find_coord_on_bezier_curve, find_coords_in_circle, find_coords_on_circle,
    find_length_of_bezier_curve, find_length_of_line, lerp_coord, Coord,
};
use crate::utils::graphics::{Color, ColorPair, Gradient};
use crate::utils::round_half_even;

const STAR_COLORS: [Color; 3] = [
    Color {
        r: 0xfd,
        g: 0xff,
        b: 0x38,
    },
    Color {
        r: 0xff,
        g: 0xd7,
        b: 0x00,
    },
    Color {
        r: 0xff,
        g: 0xff,
        b: 0xff,
    },
];

const FINAL_STOPS: [Color; 3] = [
    Color {
        r: 0x8a,
        g: 0x00,
        b: 0x8a,
    },
    Color {
        r: 0x00,
        g: 0xd1,
        b: 0xff,
    },
    Color {
        r: 0xff,
        g: 0xff,
        b: 0xff,
    },
];

const RING_SYMBOLS: [&str; 6] = ["*", "'", "`", ".", ",", "°"];
const FORM_DELAY: u32 = 3;
const CONSUME_DELAY: u32 = 2;
const WAIT_TICKS: u32 = 50;
const FORM_SPEED: f64 = 0.5;
const CONSUME_SPEED: f64 = 0.4;
const FLING_SPEED: f64 = 0.3;
const EXPLODE_SPEED: f64 = 0.3;
const COLLAPSE_SPEED: f64 = 0.5;
const MAX_FRAMES: usize = 20_000;

#[derive(Clone, Copy)]
enum Phase {
    Forming,
    Waiting,
    Consuming,
    Collapsing,
    Exploding,
    Done,
}

#[derive(Clone, Copy)]
enum EaseKind {
    OutSine,
    InQuad,
    OutExpo,
    OutCirc,
}

struct Flyer {
    id: CharacterId,
    start: Coord,
    end: Coord,
    control: Option<Coord>,
    frame: u32,
    duration: u32,
    ease: EaseKind,
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.0
    }

    fn randrange(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() as usize) % n
        }
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.randrange(i + 1);
            items.swap(i, j);
        }
    }
}

pub struct Blackhole;

impl Blackhole {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Blackhole {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Blackhole {
    fn name(&self) -> &str {
        "blackhole"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        if term.character_count() == 0 {
            return vec![term.render_frame()];
        }

        let center = term.canvas.center();
        let radius = (term.canvas.right.min(term.canvas.top) / 3).max(1);
        let ring_base = find_coords_on_circle(
            center,
            f64::from(radius),
            (radius.saturating_mul(3)).max(1) as usize,
            false,
        );

        let mut min_col = i32::MAX;
        let mut max_col = i32::MIN;
        let mut min_row = i32::MAX;
        let mut max_row = i32::MIN;
        let mut ids: Vec<CharacterId> = Vec::new();
        for ch in term.get_characters() {
            ids.push(ch.id);
            min_col = min_col.min(ch.input_coord.column);
            max_col = max_col.max(ch.input_coord.column);
            min_row = min_row.min(ch.input_coord.row);
            max_row = max_row.max(ch.input_coord.row);
        }
        if min_col == i32::MAX {
            min_col = 1;
            max_col = 1;
            min_row = 1;
            max_row = 1;
        }
        let bounds = (min_col, max_col, min_row, max_row);
        let final_gradient = Gradient::new(&FINAL_STOPS, 12);

        term.hide_all();

        let mut rng = Rng(0xC0FFEE_u64.wrapping_add(ids.len() as u64));
        let mut pending = ids.clone();
        rng.shuffle(&mut pending);

        let mut flyers: Vec<Flyer> = Vec::new();
        let mut ages: HashMap<CharacterId, u32> = HashMap::new();
        let mut from_color: HashMap<CharacterId, Color> = HashMap::new();
        let mut phase = Phase::Forming;
        let mut delay: u32 = 0;
        let mut wait_left = WAIT_TICKS;
        let mut rot: usize = 0;
        let mut collapse_frame: u32 = 0;
        let collapse_duration = travel_frames(f64::from(radius), COLLAPSE_SPEED).max(8);
        let mut frames: Vec<String> = Vec::new();

        while frames.len() < MAX_FRAMES {
            match phase {
                Phase::Forming => {
                    if delay == 0 && !pending.is_empty() {
                        let idx = rng.randrange(pending.len());
                        let id = pending.remove(idx);
                        let dest = term
                            .get_character(id)
                            .map(|ch| ch.input_coord)
                            .unwrap_or(center);
                        let dist = find_length_of_line(center, dest);
                        if let Some(ch) = term.get_character_mut(id) {
                            ch.motion.current_coord = center;
                            ch.is_visible = true;
                            let sym = ch.input_symbol.clone();
                            ch.animation
                                .set_appearance(&sym, Some(ColorPair::fg(STAR_COLORS[0])));
                        }
                        ages.insert(id, 0);
                        flyers.push(Flyer {
                            id,
                            start: center,
                            end: dest,
                            control: None,
                            frame: 0,
                            duration: travel_frames(dist, FORM_SPEED),
                            ease: EaseKind::OutSine,
                        });
                        delay = FORM_DELAY;
                    } else if delay > 0 {
                        delay -= 1;
                    }
                    step_flyers(&mut term, &mut flyers, None);
                    if pending.is_empty() && flyers.is_empty() {
                        phase = Phase::Waiting;
                        wait_left = WAIT_TICKS;
                        pending = ids.clone();
                        rng.shuffle(&mut pending);
                        delay = 0;
                    }
                }
                Phase::Waiting => {
                    rot = rot.wrapping_add(1);
                    if wait_left > 0 {
                        wait_left -= 1;
                    } else {
                        phase = Phase::Consuming;
                        delay = 0;
                    }
                }
                Phase::Consuming => {
                    rot = rot.wrapping_add(1);
                    if delay == 0 && !pending.is_empty() {
                        let id = pending.pop().unwrap();
                        let start = term
                            .get_character(id)
                            .map(|ch| ch.current_coord())
                            .unwrap_or(center);
                        let control = Coord::new(center.column, start.row);
                        let dist = find_length_of_bezier_curve(start, control, center);
                        flyers.push(Flyer {
                            id,
                            start,
                            end: center,
                            control: Some(control),
                            frame: 0,
                            duration: travel_frames(dist, CONSUME_SPEED),
                            ease: EaseKind::InQuad,
                        });
                        delay = CONSUME_DELAY;
                    } else if delay > 0 {
                        delay -= 1;
                    }
                    step_flyers(&mut term, &mut flyers, Some(false));
                    if pending.is_empty() && flyers.is_empty() {
                        phase = Phase::Collapsing;
                        collapse_frame = 0;
                        let nearby: Vec<Coord> = find_coords_in_circle(center, f64::from(radius) * 2.0)
                            .into_iter()
                            .filter(|&c| term.canvas.contains(c))
                            .collect();
                        for &id in &ids {
                            let start = term
                                .get_character(id)
                                .map(|ch| ch.current_coord())
                                .unwrap_or(center);
                            let dest = if nearby.is_empty() {
                                center
                            } else {
                                nearby[rng.randrange(nearby.len())]
                            };
                            if let Some(ch) = term.get_character_mut(id) {
                                ch.is_visible = true;
                                ch.motion.current_coord = start;
                            }
                            flyers.push(Flyer {
                                id,
                                start,
                                end: dest,
                                control: None,
                                frame: 0,
                                duration: travel_frames(find_length_of_line(start, dest), FLING_SPEED),
                                ease: EaseKind::OutExpo,
                            });
                        }
                    }
                }
                Phase::Collapsing => {
                    rot = rot.wrapping_add(1);
                    step_flyers(&mut term, &mut flyers, None);
                    if collapse_frame < collapse_duration {
                        collapse_frame += 1;
                    }
                    if collapse_frame >= collapse_duration && flyers.is_empty() {
                        phase = Phase::Exploding;
                        for &id in &ids {
                            let (start, input, age) = match term.get_character(id) {
                                Some(ch) => (
                                    ch.current_coord(),
                                    ch.input_coord,
                                    ages.get(&id).copied().unwrap_or(0),
                                ),
                                None => (center, center, 0),
                            };
                            from_color.insert(id, star_color(age));
                            if let Some(ch) = term.get_character_mut(id) {
                                ch.is_visible = true;
                            }
                            flyers.push(Flyer {
                                id,
                                start,
                                end: input,
                                control: None,
                                frame: 0,
                                duration: travel_frames(
                                    find_length_of_line(start, input),
                                    EXPLODE_SPEED,
                                ),
                                ease: EaseKind::OutCirc,
                            });
                        }
                    }
                }
                Phase::Exploding => {
                    step_flyers(&mut term, &mut flyers, None);
                    if flyers.is_empty() {
                        phase = Phase::Done;
                    }
                }
                Phase::Done => {}
            }

            age_visible(&mut ages, &ids);
            match phase {
                Phase::Exploding | Phase::Done => {
                    color_explode(&mut term, &ids, &flyers, &from_color, &final_gradient, bounds);
                }
                _ => {
                    color_stars(&mut term, &ages);
                }
            }

            let ring = match phase {
                Phase::Collapsing => shrinking_ring(
                    center,
                    radius,
                    collapse_frame,
                    collapse_duration,
                    rot,
                ),
                Phase::Exploding | Phase::Done => Vec::new(),
                _ => ring_sprites(&ring_base, rot),
            };
            emit_frame(&mut term, &ring, &mut frames);

            if matches!(phase, Phase::Done) {
                break;
            }
        }

        if !matches!(phase, Phase::Done) {
            for &id in &ids {
                let input = term
                    .get_character(id)
                    .map(|ch| ch.input_coord)
                    .unwrap_or(center);
                let color = mapped_final(input, bounds, &final_gradient);
                if let Some(ch) = term.get_character_mut(id) {
                    ch.motion.current_coord = input;
                    ch.is_visible = true;
                    let sym = ch.input_symbol.clone();
                    ch.animation
                        .set_appearance(&sym, Some(ColorPair::fg(color)));
                }
            }
            frames.push(term.render_frame());
        }

        frames
    }
}

fn travel_frames(dist: f64, speed: f64) -> u32 {
    if !dist.is_finite() || dist <= 0.0 {
        return 1;
    }
    let raw = if speed <= 0.0 { dist } else { dist / speed };
    round_half_even(raw).max(1) as u32
}

fn ease(kind: EaseKind, t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    match kind {
        EaseKind::OutSine => (t * std::f64::consts::FRAC_PI_2).sin(),
        EaseKind::InQuad => t * t,
        EaseKind::OutExpo => {
            if t >= 1.0 {
                1.0
            } else {
                1.0 - 2f64.powf(-10.0 * t)
            }
        }
        EaseKind::OutCirc => (1.0 - (t - 1.0) * (t - 1.0)).sqrt(),
    }
}

fn step_flyers(term: &mut Terminal, flyers: &mut Vec<Flyer>, hide_on_complete: Option<bool>) {
    for flyer in flyers.iter_mut() {
        flyer.frame = flyer.frame.saturating_add(1);
        let t = if flyer.duration == 0 {
            1.0
        } else {
            (f64::from(flyer.frame) / f64::from(flyer.duration)).clamp(0.0, 1.0)
        };
        let te = ease(flyer.ease, t);
        let pos = match flyer.control {
            Some(ctrl) => find_coord_on_bezier_curve(flyer.start, ctrl, flyer.end, te),
            None => lerp_coord(flyer.start, flyer.end, te),
        };
        let done = flyer.frame >= flyer.duration;
        if let Some(ch) = term.get_character_mut(flyer.id) {
            ch.motion.current_coord = if done { flyer.end } else { pos };
            if done {
                if let Some(visible) = hide_on_complete {
                    ch.is_visible = visible;
                }
            }
        }
    }
    flyers.retain(|f| f.frame < f.duration);
}

fn star_color(age: u32) -> Color {
    let idx = (age / 40) as usize;
    if idx >= STAR_COLORS.len() {
        STAR_COLORS[STAR_COLORS.len() - 1]
    } else {
        STAR_COLORS[idx]
    }
}

fn age_visible(ages: &mut HashMap<CharacterId, u32>, ids: &[CharacterId]) {
    for id in ids {
        if let Some(age) = ages.get_mut(id) {
            *age = age.saturating_add(1);
        }
    }
}

fn color_stars(term: &mut Terminal, ages: &HashMap<CharacterId, u32>) {
    let ids: Vec<CharacterId> = term.get_characters().iter().map(|ch| ch.id).collect();
    for id in ids {
        let age = ages.get(&id).copied().unwrap_or(0);
        let color = star_color(age);
        if let Some(ch) = term.get_character_mut(id) {
            if !ch.is_visible {
                continue;
            }
            let sym = ch.input_symbol.clone();
            ch.animation
                .set_appearance(&sym, Some(ColorPair::fg(color)));
        }
    }
}

fn mapped_final(coord: Coord, bounds: (i32, i32, i32, i32), gradient: &Gradient) -> Color {
    let (min_col, max_col, min_row, max_row) = bounds;
    let cw = f64::from((max_col - min_col).max(1));
    let rh = f64::from((max_row - min_row).max(1));
    let col_p = f64::from(coord.column - min_col) / cw;
    let row_p = f64::from(max_row - coord.row) / rh;
    gradient
        .mapped_color(((col_p + row_p) * 0.5).clamp(0.0, 1.0))
        .unwrap_or(Color::rgb(255, 255, 255))
}

fn color_explode(
    term: &mut Terminal,
    ids: &[CharacterId],
    flyers: &[Flyer],
    from_color: &HashMap<CharacterId, Color>,
    final_gradient: &Gradient,
    bounds: (i32, i32, i32, i32),
) {
    let mut progress: HashMap<CharacterId, f64> = HashMap::new();
    for flyer in flyers {
        let t = if flyer.duration == 0 {
            1.0
        } else {
            (f64::from(flyer.frame) / f64::from(flyer.duration)).clamp(0.0, 1.0)
        };
        progress.insert(flyer.id, ease(flyer.ease, t));
    }
    for &id in ids {
        let (input, visible) = match term.get_character(id) {
            Some(ch) => (ch.input_coord, ch.is_visible),
            None => continue,
        };
        if !visible {
            continue;
        }
        let t = progress.get(&id).copied().unwrap_or(1.0);
        let from = from_color
            .get(&id)
            .copied()
            .unwrap_or(Color::rgb(255, 255, 255));
        let to = mapped_final(input, bounds, final_gradient);
        let blend = Gradient::new(&[from, to], 10);
        let color = blend.mapped_color(t).unwrap_or(to);
        if let Some(ch) = term.get_character_mut(id) {
            let sym = ch.input_symbol.clone();
            ch.animation
                .set_appearance(&sym, Some(ColorPair::fg(color)));
        }
    }
}

fn ring_sprites(base: &[Coord], rot: usize) -> Vec<(Coord, &'static str)> {
    let n = base.len();
    if n == 0 {
        return Vec::new();
    }
    (0..n)
        .map(|i| (base[(i + rot) % n], RING_SYMBOLS[i % RING_SYMBOLS.len()]))
        .collect()
}

fn shrinking_ring(
    center: Coord,
    radius: i32,
    frame: u32,
    duration: u32,
    rot: usize,
) -> Vec<(Coord, &'static str)> {
    let t = if duration == 0 {
        1.0
    } else {
        (f64::from(frame) / f64::from(duration)).clamp(0.0, 1.0)
    };
    let eased = -((t * std::f64::consts::PI).cos()) * 0.5 + 0.5;
    let r = f64::from(radius) * (1.0 - eased);
    if r < 0.5 {
        return vec![(center, "*")];
    }
    let n = (r * 3.0).round().max(1.0) as usize;
    let base = find_coords_on_circle(center, r, n, false);
    ring_sprites(&base, rot)
}

fn emit_frame(term: &mut Terminal, ring: &[(Coord, &'static str)], frames: &mut Vec<String>) {
    let _ = term.render_frame();
    for &(coord, sym) in ring {
        let blocked = term
            .get_characters()
            .iter()
            .any(|ch| ch.is_visible && ch.current_coord() == coord);
        if !blocked && term.canvas.contains(coord) {
            term.canvas.put(coord, CharacterVisual::new(sym, None));
        }
    }
    frames.push(term.canvas.render());
}
