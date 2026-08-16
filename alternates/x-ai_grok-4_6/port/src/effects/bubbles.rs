use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{self, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};
use crate::utils::round_half_even;

const BUBBLE_SPEED: f64 = 0.1;
const BUBBLE_DELAY: u32 = 50;
const HOME_SPEED: f64 = 0.3;
const POP_HOLD: u32 = 8;
const SHEEN_HOLD: usize = 4;
const MAX_FRAMES: usize = 20_000;
const POP_GLYPHS: [&str; 4] = ["*", "e", "o", "."];

pub struct Bubbles;

impl Bubbles {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Bubbles {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Bubbles {
    fn name(&self) -> &str {
        "bubbles"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        run_bubbles(input)
    }
}

fn color(hex: &str) -> Color {
    Color::from_hex(hex).unwrap_or(Color::rgb(255, 255, 255))
}

fn ease_in_out_sine(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    -(t * std::f64::consts::PI).cos() * 0.5 + 0.5
}

fn paint(term: &mut Terminal, id: CharacterId, at: Coord, symbol: &str, tint: Color) {
    if let Some(ch) = term.get_character_mut(id) {
        ch.motion.current_coord = at;
        ch.animation
            .set_appearance(symbol, Some(ColorPair::fg(tint)));
    }
}

fn mapped_final_color(grad: &Gradient, coord: Coord, left: i32, right: i32, bottom: i32, top: i32) -> Color {
    let w = (right - left).max(0) as f64;
    let h = (top - bottom).max(0) as f64;
    let px = if w == 0.0 {
        0.0
    } else {
        (coord.column - left) as f64 / w
    };
    let py = if h == 0.0 {
        0.0
    } else {
        (coord.row - bottom) as f64 / h
    };
    grad.mapped_color(((px + py) * 0.5).clamp(0.0, 1.0))
        .unwrap_or(color("48c9b0"))
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed | 1 }
    }

    fn next(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        (x >> 32) as u32
    }

    fn gen_inclusive(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let span = (hi as i64 - lo as i64 + 1) as u32;
        lo.saturating_add((self.next() % span) as i32)
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.next() as usize % (i + 1);
            items.swap(i, j);
        }
    }
}

struct CharSnap {
    id: CharacterId,
    symbol: String,
    input: Coord,
    final_color: Color,
}

struct Bubble {
    members: Vec<usize>,
    circle: Vec<Coord>,
    origin: Coord,
    y: f64,
    dest_y: f64,
    sheen_tick: usize,
}

struct Flyer {
    idx: usize,
    start: Coord,
    t: f64,
    dist: f64,
    pop_age: u32,
}

fn bubble_coord(circle: Coord, origin: Coord, y: f64) -> Coord {
    let cy = round_half_even(y) as i32;
    Coord {
        column: circle.column,
        row: circle.row + (cy - origin.row),
    }
}

fn run_bubbles(input: &str) -> Vec<String> {
    let mut term = Terminal::from_input(input, TerminalConfig::default());
    term.hide_all();

    if term.character_count() == 0 {
        return vec![term.render_frame()];
    }

    let bubble_gradient = Gradient::new(
        &[
            color("d1f2eb"),
            color("9aecdb"),
            color("76d7c4"),
            color("48c9b0"),
        ],
        8,
    );
    let final_gradient = Gradient::new(&[color("d1f2eb"), color("48c9b0")], 12);
    let pop_color = color("ffffff");

    let text_left = term
        .get_characters()
        .iter()
        .map(|c| c.input_coord.column)
        .min()
        .unwrap_or(term.canvas.left);
    let text_right = term
        .get_characters()
        .iter()
        .map(|c| c.input_coord.column)
        .max()
        .unwrap_or(term.canvas.right);
    let text_bottom = term
        .get_characters()
        .iter()
        .map(|c| c.input_coord.row)
        .min()
        .unwrap_or(term.canvas.bottom);
    let text_top = term
        .get_characters()
        .iter()
        .map(|c| c.input_coord.row)
        .max()
        .unwrap_or(term.canvas.top);

    let mut snaps: Vec<CharSnap> = term
        .get_characters()
        .iter()
        .map(|ch| CharSnap {
            id: ch.id,
            symbol: ch.input_symbol.clone(),
            input: ch.input_coord,
            final_color: mapped_final_color(
                &final_gradient,
                ch.input_coord,
                text_left,
                text_right,
                text_bottom,
                text_top,
            ),
        })
        .collect();

    let canvas_left = term.canvas.left;
    let canvas_right = term.canvas.right;
    let canvas_top = term.canvas.top;
    let canvas_bottom = term.canvas.bottom;

    let mut rng = SimpleRng::new(0x00B0_BB1E);
    rng.shuffle(&mut snaps);

    let mut groups: Vec<Vec<usize>> = Vec::new();
    {
        let mut remaining: Vec<usize> = (0..snaps.len()).collect();
        while !remaining.is_empty() {
            let take = (rng.gen_inclusive(5, 20) as usize).min(remaining.len());
            groups.push(remaining.drain(0..take).collect());
        }
    }

    let mut pending: Vec<Bubble> = Vec::new();
    for group in groups {
        if group.is_empty() {
            continue;
        }
        let lowest_row = group
            .iter()
            .map(|&i| snaps[i].input.row)
            .min()
            .unwrap_or(canvas_bottom);
        let origin = Coord {
            column: rng.gen_inclusive(canvas_left, canvas_right),
            row: canvas_top,
        };
        let radius = round_half_even(group.len() as f64 / 5.0).max(1) as f64;
        let circle = geometry::find_coords_on_circle(origin, radius, group.len(), false);
        pending.push(Bubble {
            members: group,
            circle,
            origin,
            y: f64::from(origin.row),
            dest_y: f64::from(lowest_row),
            sheen_tick: 0,
        });
    }

    let mut active: Vec<Bubble> = Vec::new();
    let mut flyers: Vec<Flyer> = Vec::new();
    let mut delay = 0u32;
    let mut next_pending = 0usize;
    let mut frames: Vec<String> = Vec::new();
    let sheen_len = bubble_gradient.len().max(1);

    for _ in 0..MAX_FRAMES {
        if delay == 0 && next_pending < pending.len() {
            let mut bubble = pending[next_pending].clone_meta();
            bubble.members = pending[next_pending].members.clone();
            bubble.circle = pending[next_pending].circle.clone();
            next_pending += 1;

            for (offset, &idx) in bubble.members.iter().enumerate() {
                let at = bubble
                    .circle
                    .get(offset)
                    .copied()
                    .unwrap_or(bubble.origin);
                term.set_character_visibility(snaps[idx].id, true);
                let tint = bubble_gradient
                    .get(offset % sheen_len)
                    .unwrap_or(color("76d7c4"));
                paint(&mut term, snaps[idx].id, at, &snaps[idx].symbol, tint);
            }
            active.push(bubble);
            delay = BUBBLE_DELAY;
        } else if delay > 0 {
            delay -= 1;
        }

        let mut popped_now: Vec<Bubble> = Vec::new();
        for bubble in &mut active {
            if bubble.y > bubble.dest_y {
                bubble.y = (bubble.y - BUBBLE_SPEED).max(bubble.dest_y);
            }
            bubble.sheen_tick = bubble.sheen_tick.wrapping_add(1);
            for (offset, &idx) in bubble.members.iter().enumerate() {
                let circ = bubble
                    .circle
                    .get(offset)
                    .copied()
                    .unwrap_or(bubble.origin);
                let at = bubble_coord(circ, bubble.origin, bubble.y);
                let tint = bubble_gradient
                    .get((bubble.sheen_tick / SHEEN_HOLD + offset) % sheen_len)
                    .unwrap_or(color("76d7c4"));
                paint(&mut term, snaps[idx].id, at, &snaps[idx].symbol, tint);
            }
        }
        let mut keep = Vec::new();
        for bubble in active.drain(..) {
            if bubble.y <= bubble.dest_y {
                popped_now.push(bubble);
            } else {
                keep.push(bubble);
            }
        }
        active = keep;

        for bubble in popped_now {
            for (offset, &idx) in bubble.members.iter().enumerate() {
                let circ = bubble
                    .circle
                    .get(offset)
                    .copied()
                    .unwrap_or(bubble.origin);
                let start = bubble_coord(circ, bubble.origin, bubble.y);
                let dist = geometry::distance(start, snaps[idx].input);
                flyers.push(Flyer {
                    idx,
                    start,
                    t: 0.0,
                    dist,
                    pop_age: 0,
                });
            }
        }

        for flyer in &mut flyers {
            if flyer.dist < 0.001 {
                flyer.t = 1.0;
            } else if flyer.t < 1.0 {
                flyer.t = (flyer.t + HOME_SPEED / flyer.dist).min(1.0);
            }
            flyer.pop_age = flyer.pop_age.saturating_add(1);
            let eased = ease_in_out_sine(flyer.t);
            let at = geometry::lerp_coord(flyer.start, snaps[flyer.idx].input, eased);
            let stage = (flyer.pop_age / POP_HOLD) as usize;
            let (symbol, tint) = if stage < POP_GLYPHS.len() {
                (POP_GLYPHS[stage], pop_color)
            } else {
                (snaps[flyer.idx].symbol.as_str(), snaps[flyer.idx].final_color)
            };
            let symbol = symbol.to_string();
            paint(
                &mut term,
                snaps[flyer.idx].id,
                at,
                &symbol,
                tint,
            );
        }

        frames.push(term.render_frame());

        let settled = flyers.iter().all(|f| f.t >= 1.0);
        if next_pending >= pending.len() && active.is_empty() && settled {
            if let Some(last) = frames.last().cloned() {
                for _ in 0..4 {
                    frames.push(last.clone());
                }
            }
            break;
        }
    }

    if frames.is_empty() {
        frames.push(term.render_frame());
    }
    frames
}

impl Bubble {
    fn clone_meta(&self) -> Self {
        Self {
            members: Vec::new(),
            circle: Vec::new(),
            origin: self.origin,
            y: self.y,
            dest_y: self.dest_y,
            sheen_tick: 0,
        }
    }
}
