//! Binary representations of each input character move through the
//! terminal towards the home coordinate of the character.

use super::Effect;
use crate::engine::canvas::Canvas;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{find_coords_on_line, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

const BINARY_HEX: [&str; 3] = ["00d500", "007500", "003400"];
const FINAL_HEX: [&str; 3] = ["8A008A", "00D1FF", "FFFFFF"];
const REVEAL_BITS: usize = 8;
const WIPE_FRAMES: usize = 10;
const HOLD_FRAMES: usize = 16;
const GROUP_SIZE: usize = 8;
const GROUP_GAP: usize = 4;

pub struct Binarypath;

impl Binarypath {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Binarypath {
    fn name(&self) -> &str {
        "binarypath"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        term.hide_all();

        if term.character_count() == 0 {
            return vec![term.render_frame()];
        }

        let binary_colors = [
            color_or(BINARY_HEX[0], 0x00, 0xd5, 0x00),
            color_or(BINARY_HEX[1], 0x00, 0x75, 0x00),
            color_or(BINARY_HEX[2], 0x00, 0x34, 0x00),
        ];
        let final_stops = [
            color_or(FINAL_HEX[0], 0x8a, 0x00, 0x8a),
            color_or(FINAL_HEX[1], 0x00, 0xd1, 0xff),
            color_or(FINAL_HEX[2], 0xff, 0xff, 0xff),
        ];
        let final_gradient = Gradient::new(&final_stops, 12);
        let white = Color::rgb(255, 255, 255);

        let snapshot: Vec<(CharacterId, String, Coord)> = term
            .get_characters()
            .iter()
            .map(|ch| (ch.id, ch.input_symbol.clone(), ch.input_coord))
            .collect();

        let min_row = snapshot.iter().map(|(_, _, c)| c.row).min().unwrap_or(1);
        let max_row = snapshot.iter().map(|(_, _, c)| c.row).max().unwrap_or(1);
        let row_span = (max_row - min_row).max(1) as f64;
        let center = term.canvas.center();

        let mut plans: Vec<CharPlan> = Vec::with_capacity(snapshot.len());
        for (index, (id, symbol, home)) in snapshot.into_iter().enumerate() {
            let progress = (home.row - min_row) as f64 / row_span;
            let final_color = final_gradient
                .mapped_color(progress)
                .unwrap_or(final_stops[final_stops.len() - 1]);
            let binary_color = binary_colors[mix(id.0, 7) as usize % binary_colors.len()];
            let start = group_start(&term.canvas, (index / GROUP_SIZE) as u32, (index % GROUP_SIZE) as i32);
            let path = travel_path(start, center, home);
            plans.push(CharPlan {
                id,
                symbol,
                bits: symbol_bits(&plans_symbol_ref(home, index, &path, start)),
                path,
                binary_color,
                final_color,
                launch_frame: (index / GROUP_SIZE) * GROUP_GAP,
            });
            // `symbol_bits` needs the actual glyph; fix the placeholder above.
            let last = plans.len() - 1;
            plans[last].bits = symbol_bits(&plans[last].symbol);
            let _ = (home, start);
        }

        // Rebuild bits correctly without the dummy helper dance.
        for plan in &mut plans {
            plan.bits = symbol_bits(&plan.symbol);
        }

        let mut total = HOLD_FRAMES;
        for plan in &plans {
            let end = plan.launch_frame + plan.travel_len() + REVEAL_BITS + WIPE_FRAMES;
            if end + HOLD_FRAMES > total {
                total = end + HOLD_FRAMES;
            }
        }

        let mut frames = Vec::with_capacity(total.max(1));
        for frame in 0..total {
            for plan in &plans {
                apply_frame(&mut term, plan, frame, white);
            }
            frames.push(term.render_frame());
        }
        if frames.is_empty() {
            frames.push(term.render_frame());
        }
        frames
    }
}

struct CharPlan {
    id: CharacterId,
    symbol: String,
    bits: [char; 8],
    path: Vec<Coord>,
    binary_color: Color,
    final_color: Color,
    launch_frame: usize,
}

impl CharPlan {
    fn travel_len(&self) -> usize {
        self.path.len().max(12)
    }
}

fn apply_frame(term: &mut Terminal, plan: &CharPlan, frame: usize, white: Color) {
    if frame < plan.launch_frame {
        term.set_character_visibility(plan.id, false);
        return;
    }

    let local = frame - plan.launch_frame;
    let travel = plan.travel_len();
    let Some(ch) = term.get_character_mut(plan.id) else {
        return;
    };
    ch.is_visible = true;

    if local < travel {
        let denom = (travel - 1).max(1) as f64;
        let t = in_quad((local as f64 / denom).clamp(0.0, 1.0));
        let idx = if plan.path.is_empty() {
            0
        } else {
            let last = plan.path.len() - 1;
            (t * last as f64).round().clamp(0.0, last as f64) as usize
        };
        if let Some(coord) = plan.path.get(idx).copied() {
            ch.motion.current_coord = coord;
        }
        let bit = plan.bits[(local / 3) % plan.bits.len()];
        let glyph = bit.to_string();
        ch.animation
            .set_appearance(&glyph, Some(ColorPair::fg(plan.binary_color)));
        return;
    }

    ch.motion.current_coord = plan
        .path
        .last()
        .copied()
        .unwrap_or(ch.input_coord);

    let after = local - travel;
    if after < REVEAL_BITS {
        let bit = plan.bits[after % plan.bits.len()];
        let glyph = bit.to_string();
        ch.animation
            .set_appearance(&glyph, Some(ColorPair::fg(plan.binary_color)));
        return;
    }

    let wipe_at = after - REVEAL_BITS;
    if wipe_at < WIPE_FRAMES {
        let wipe = Gradient::new(&[plan.final_color, white, plan.final_color], 5);
        let p = if WIPE_FRAMES <= 1 {
            1.0
        } else {
            wipe_at as f64 / (WIPE_FRAMES - 1) as f64
        };
        let color = wipe.mapped_color(p).unwrap_or(plan.final_color);
        ch.animation
            .set_appearance(&plan.symbol, Some(ColorPair::fg(color)));
        return;
    }

    ch.animation
        .set_appearance(&plan.symbol, Some(ColorPair::fg(plan.final_color)));
}

fn travel_path(start: Coord, mid: Coord, home: Coord) -> Vec<Coord> {
    let mut path = find_coords_on_line(start, mid);
    let home_leg = find_coords_on_line(mid, home);
    if path.last() == home_leg.first() {
        path.extend(home_leg.into_iter().skip(1));
    } else {
        path.extend(home_leg);
    }
    if path.is_empty() {
        path.push(home);
    }
    path
}

fn group_start(canvas: &Canvas, group: u32, slot: i32) -> Coord {
    let w = canvas.width.max(1) as u32;
    let h = canvas.height.max(1) as u32;
    match mix(group, 11) % 4 {
        0 => Coord::new(
            canvas.left + (mix(group, 22) % w) as i32 + slot,
            canvas.top + 1,
        ),
        1 => Coord::new(
            canvas.left + (mix(group, 33) % w) as i32 + slot,
            canvas.bottom - 1,
        ),
        2 => Coord::new(
            canvas.left - 1,
            canvas.bottom + (mix(group, 44) % h) as i32 + slot,
        ),
        _ => Coord::new(
            canvas.right + 1,
            canvas.bottom + (mix(group, 55) % h) as i32 + slot,
        ),
    }
}

fn symbol_bits(symbol: &str) -> [char; 8] {
    let value = symbol.chars().next().map(|c| c as u32).unwrap_or(0) & 0xff;
    let mut bits = ['0'; 8];
    for (i, bit) in bits.iter_mut().enumerate() {
        if value & (1 << (7 - i)) != 0 {
            *bit = '1';
        }
    }
    bits
}

fn color_or(hex: &str, r: u8, g: u8, b: u8) -> Color {
    Color::from_hex(hex).unwrap_or(Color::rgb(r, g, b))
}

fn in_quad(t: f64) -> f64 {
    t * t
}

fn mix(id: u32, salt: u32) -> u32 {
    id.wrapping_mul(1664525).wrapping_add(1013904223).wrapping_add(salt)
}

fn plans_symbol_ref(_home: Coord, _index: usize, _path: &Vec<Coord>, _start: Coord) -> String {
    String::new()
}
