use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};
use crate::utils::round_half_even;

/// Default TTE print-head carriage-return speed (cells per frame).
const PRINT_HEAD_RETURN_SPEED: f64 = 1.25;
/// Default TTE characters committed per idle tick.
const PRINT_SPEED: usize = 1;
/// Default TTE final gradient stops.
const GRADIENT_STEPS: usize = 12;

pub struct Print;

impl Print {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Print {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Print {
    fn name(&self) -> &str {
        "print"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        if term.character_count() == 0 {
            return vec![term.render_frame()];
        }

        let bottom = term.canvas.bottom;
        let left = term.canvas.left;
        let final_colors = build_final_colors(&term);

        for ch in term.get_characters_mut() {
            ch.is_visible = false;
            ch.motion.current_coord = Coord::new(ch.input_coord.column, bottom);
        }

        let mut pending = group_rows(&term);
        if pending.is_empty() {
            return vec![term.render_frame()];
        }

        let mut processed: Vec<RowState> = Vec::new();
        let mut current = take_row(&mut pending);
        let mut head = head_at_row_start(&term, &current, bottom);
        let mut lives: Vec<Live> = Vec::new();
        let mut phase = Phase::Print;
        let mut frames: Vec<String> = Vec::new();
        let max_frames = term
            .character_count()
            .saturating_mul(24)
            .saturating_add(pending.len().saturating_mul(256))
            .saturating_add(64);

        while frames.len() < max_frames {
            if !lives.is_empty() {
                frames.push(render(&mut term, head));
                step_lives(&mut term, &mut lives);
                continue;
            }

            match phase {
                Phase::Print => {
                    let has_untyped = current
                        .as_ref()
                        .is_some_and(|row| !row.untyped.is_empty());
                    if has_untyped {
                        if let Some(row) = current.as_mut() {
                            spawn_batch(
                                &mut term,
                                row,
                                &mut lives,
                                &final_colors,
                                &mut head,
                                bottom,
                            );
                        }
                        frames.push(render(&mut term, head));
                        step_lives(&mut term, &mut lives);
                    } else {
                        let start = last_typed_column(&term, current.as_ref()).unwrap_or(left);
                        let distance = f64::from((start - left).unsigned_abs());
                        let total = ((distance / PRINT_HEAD_RETURN_SPEED).ceil() as usize).max(1);
                        head = Some(Coord::new(start, bottom));
                        phase = Phase::Return {
                            start,
                            tick: 0,
                            total,
                        };
                    }
                }
                Phase::Return {
                    start,
                    tick,
                    total,
                } => {
                    if tick >= total {
                        if pending.is_empty() {
                            frames.push(render(&mut term, None));
                            break;
                        }
                        phase = Phase::Feed;
                    } else {
                        let denom = if total <= 1 { 1.0 } else { (total - 1) as f64 };
                        let t = in_out_quad(tick as f64 / denom);
                        let col = round_half_even(
                            f64::from(start) + (f64::from(left) - f64::from(start)) * t,
                        ) as i32;
                        head = Some(Coord::new(col, bottom));
                        frames.push(render(&mut term, head));
                        phase = Phase::Return {
                            start,
                            tick: tick + 1,
                            total,
                        };
                    }
                }
                Phase::Feed => {
                    if let Some(row) = current.take() {
                        processed.push(row);
                    }
                    move_rows_up(&mut term, &processed);
                    if pending.is_empty() {
                        frames.push(render(&mut term, None));
                        break;
                    }
                    current = take_row(&mut pending);
                    head = head_at_row_start(&term, &current, bottom);
                    phase = Phase::Print;
                    frames.push(render(&mut term, head));
                }
            }
        }

        if frames.is_empty() {
            frames.push(render(&mut term, None));
        }
        frames
    }
}

#[derive(Clone, Copy)]
enum Phase {
    Print,
    Return { start: i32, tick: usize, total: usize },
    Feed,
}

struct RowState {
    untyped: Vec<CharacterId>,
    typed: Vec<CharacterId>,
}

struct Live {
    id: CharacterId,
    sequence: Vec<(String, Option<Color>)>,
    pos: usize,
}

fn group_rows(term: &Terminal) -> Vec<RowState> {
    let mut items: Vec<(i32, i32, CharacterId)> = term
        .get_characters()
        .iter()
        .map(|ch| (ch.input_coord.row, ch.input_coord.column, ch.id))
        .collect();
    items.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

    let mut rows: Vec<RowState> = Vec::new();
    let mut current_row: Option<i32> = None;
    for (row, _, id) in items {
        if current_row != Some(row) {
            rows.push(RowState {
                untyped: Vec::new(),
                typed: Vec::new(),
            });
            current_row = Some(row);
        }
        if let Some(last) = rows.last_mut() {
            last.untyped.push(id);
        }
    }
    rows
}

fn build_final_colors(term: &Terminal) -> Vec<Color> {
    let chars = term.get_characters();
    let mut colors = vec![Color::rgb(255, 255, 255); term.character_count()];
    if chars.is_empty() {
        return colors;
    }

    let text_left = chars.iter().map(|c| c.input_coord.column).min().unwrap_or(1);
    let text_right = chars.iter().map(|c| c.input_coord.column).max().unwrap_or(1);
    let text_bottom = chars.iter().map(|c| c.input_coord.row).min().unwrap_or(1);
    let text_top = chars.iter().map(|c| c.input_coord.row).max().unwrap_or(1);

    let stops = [
        Color::from_hex("02b8bd").unwrap_or(Color::rgb(0x02, 0xb8, 0xbd)),
        Color::from_hex("69d6d2").unwrap_or(Color::rgb(0x69, 0xd6, 0xd2)),
        Color::from_hex("ffffff").unwrap_or(Color::rgb(0xff, 0xff, 0xff)),
    ];
    let gradient = Gradient::new(&stops, GRADIENT_STEPS);
    let denom = f64::from(text_top - text_bottom) + f64::from(text_right - text_left);

    for ch in chars {
        let progress = if denom == 0.0 {
            0.0
        } else {
            (f64::from(ch.input_coord.row - text_bottom)
                + f64::from(ch.input_coord.column - text_left))
                / denom
        };
        if let Some(color) = gradient.mapped_color(progress) {
            if let Some(slot) = colors.get_mut(ch.id.0 as usize) {
                *slot = color;
            }
        }
    }
    colors
}

fn take_row(pending: &mut Vec<RowState>) -> Option<RowState> {
    if pending.is_empty() {
        None
    } else {
        Some(pending.remove(0))
    }
}

fn head_at_row_start(term: &Terminal, row: &Option<RowState>, bottom: i32) -> Option<Coord> {
    let id = row.as_ref().and_then(|r| r.untyped.first()).copied()?;
    let ch = term.get_character(id)?;
    Some(Coord::new(ch.input_coord.column, bottom))
}

fn last_typed_column(term: &Terminal, row: Option<&RowState>) -> Option<i32> {
    let id = row.and_then(|r| r.typed.last()).copied()?;
    term.get_character(id).map(|ch| ch.input_coord.column)
}

fn spawn_batch(
    term: &mut Terminal,
    row: &mut RowState,
    lives: &mut Vec<Live>,
    final_colors: &[Color],
    head: &mut Option<Coord>,
    bottom: i32,
) {
    let n = PRINT_SPEED.min(row.untyped.len());
    for _ in 0..n {
        let id = row.untyped.remove(0);
        row.typed.push(id);
        let (symbol, cid, column) = match term.get_character(id) {
            Some(ch) => (
                ch.input_symbol.clone(),
                ch.character_id,
                ch.input_coord.column,
            ),
            None => continue,
        };
        let color = final_colors
            .get(id.0 as usize)
            .copied()
            .unwrap_or(Color::rgb(255, 255, 255));
        let sequence = color_code_sequence(cid, &symbol, color);
        if let Some((first_sym, first_color)) = sequence.first() {
            if let Some(ch) = term.get_character_mut(id) {
                ch.is_visible = true;
                ch.animation
                    .set_appearance(first_sym, first_color.map(ColorPair::fg));
            }
        }
        lives.push(Live {
            id,
            sequence,
            pos: 0,
        });
        *head = Some(Coord::new(column + 1, bottom));
    }
}

fn color_code_sequence(id: u32, symbol: &str, color: Color) -> Vec<(String, Option<Color>)> {
    let mut seq = Vec::with_capacity(9);
    seq.push((format!("{:08x}", id.wrapping_mul(0x9E37_79B1)), None));
    seq.push((format!("{:08x}", id.wrapping_mul(0x85EB_CA6B)), None));
    seq.push((format!("{:08x}", id.wrapping_mul(0xC2B2_AE3D)), None));
    for _ in 0..5 {
        seq.push(("*".to_string(), Some(color)));
    }
    seq.push((symbol.to_string(), Some(color)));
    seq
}

fn step_lives(term: &mut Terminal, lives: &mut Vec<Live>) {
    let mut keep = Vec::with_capacity(lives.len());
    for mut live in lives.drain(..) {
        live.pos += 1;
        if live.pos < live.sequence.len() {
            let (ref symbol, color) = live.sequence[live.pos];
            if let Some(ch) = term.get_character_mut(live.id) {
                ch.animation
                    .set_appearance(symbol, color.map(ColorPair::fg));
            }
            keep.push(live);
        }
    }
    *lives = keep;
}

fn move_rows_up(term: &mut Terminal, rows: &[RowState]) {
    for row in rows {
        for id in row.typed.iter().chain(row.untyped.iter()) {
            if let Some(ch) = term.get_character_mut(*id) {
                ch.motion.current_coord.row += 1;
            }
        }
    }
}

fn render(term: &mut Terminal, head: Option<Coord>) -> String {
    let _ = term.render_frame();
    if let Some(coord) = head {
        if term.canvas.contains(coord) {
            term.canvas.put(coord, CharacterVisual::new("█", None));
        }
    }
    term.canvas.render()
}

fn in_out_quad(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        2.0 * t * t
    } else {
        let x = -2.0 * t + 2.0;
        1.0 - (x * x) / 2.0
    }
}
