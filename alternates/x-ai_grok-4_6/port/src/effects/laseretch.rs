use std::collections::{HashMap, HashSet};

use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{self, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// A laser etches characters onto the terminal, then they cool to their
/// final color. The beam is traced from the canvas top-right corner along
/// a nearest-neighbour tour of the input glyphs (Python `laseretch`).
pub struct Laseretch;

impl Laseretch {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Laseretch {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Laseretch {
    fn name(&self) -> &str {
        "laseretch"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        if term.character_count() == 0 {
            return vec![term.render_frame()];
        }

        term.hide_all();

        let infos: Vec<EtchChar> = term
            .get_characters()
            .iter()
            .map(|ch| EtchChar {
                id: ch.id,
                coord: ch.input_coord,
                symbol: ch.input_symbol.clone(),
                fg: ch.input_fg,
                bg: ch.input_bg,
            })
            .collect();

        let min_col = infos.iter().map(|c| c.coord.column).min().unwrap_or(1);
        let max_col = infos.iter().map(|c| c.coord.column).max().unwrap_or(1);
        let min_row = infos.iter().map(|c| c.coord.row).min().unwrap_or(1);
        let max_row = infos.iter().map(|c| c.coord.row).max().unwrap_or(1);

        let laser = hex("e63e31");
        let cool = Gradient::new(&[laser, hex("ff9d00"), hex("6e6e6e"), hex("feffff")], 4);
        let final_grad = Gradient::new(&[hex("8a8a8a"), hex("ffffff")], 12);
        let cool_len = cool.len().max(1);
        let cool_total = cool_len * COOL_HOLD;

        let finals: Vec<Color> = infos
            .iter()
            .map(|info| {
                info.fg.unwrap_or_else(|| {
                    vertical_color(info.coord, min_col, max_col, min_row, max_row, &final_grad)
                        .unwrap_or(hex("ffffff"))
                })
            })
            .collect();

        let etchable: Vec<usize> = infos
            .iter()
            .enumerate()
            .filter(|(_, info)| is_etchable(info))
            .map(|(i, _)| i)
            .collect();

        if etchable.is_empty() {
            apply_finals(&mut term, &infos, &finals);
            term.show_all();
            return vec![term.render_frame()];
        }

        let emitter = Coord {
            column: term.canvas.right,
            row: term.canvas.top,
        };
        let order = nearest_neighbor(&infos, &etchable, emitter);

        let mut path: Vec<Coord> = Vec::new();
        let mut cursor = emitter;
        for &idx in &order {
            let dest = infos[idx].coord;
            let mut line = geometry::find_coords_on_line(cursor, dest);
            if !path.is_empty() && !line.is_empty() {
                line.remove(0);
            }
            path.extend(line);
            cursor = dest;
        }
        if path.is_empty() {
            path.push(emitter);
        }

        let mut at_coord: HashMap<Coord, usize> = HashMap::new();
        for &idx in &etchable {
            at_coord.insert(infos[idx].coord, idx);
        }

        let mut age: Vec<Option<usize>> = vec![None; infos.len()];
        let mut path_i = 0usize;
        let mut laser_pos = emitter;
        let mut retract: Vec<Coord> = Vec::new();
        let mut retract_i = 0usize;
        let mut retract_ready = false;
        let mut hold = 0usize;
        let mut frames: Vec<String> = Vec::new();

        loop {
            if frames.len() >= MAX_FRAMES {
                break;
            }

            let path_done = path_i >= path.len();
            if path_done && !retract_ready {
                let mut back = geometry::find_coords_on_line(laser_pos, emitter);
                if !back.is_empty() {
                    back.remove(0);
                }
                retract = back;
                retract_ready = true;
            }
            let retract_done = retract_ready && retract_i >= retract.len();
            let cooled = all_cooled(&age, cool_total);

            if path_done && retract_done && cooled && hold >= HOLD_FRAMES {
                break;
            }

            if !path_done {
                for _ in 0..LASER_STEP {
                    if path_i >= path.len() {
                        break;
                    }
                    laser_pos = path[path_i];
                    if let Some(&idx) = at_coord.get(&laser_pos) {
                        if age[idx].is_none() {
                            age[idx] = Some(0);
                            term.set_character_visibility(infos[idx].id, true);
                        }
                    }
                    path_i += 1;
                }
            } else if !retract_done {
                for _ in 0..LASER_STEP {
                    if retract_i >= retract.len() {
                        break;
                    }
                    laser_pos = retract[retract_i];
                    retract_i += 1;
                }
            } else if cooled {
                hold += 1;
            }

            for i in 0..infos.len() {
                let Some(a) = age[i] else {
                    continue;
                };
                let color = if a >= cool_total {
                    finals[i]
                } else {
                    cool.get(a / COOL_HOLD).unwrap_or(laser)
                };
                let pair = ColorPair::new(Some(color), infos[i].bg);
                if let Some(ch) = term.get_character_mut(infos[i].id) {
                    ch.animation
                        .set_appearance(&infos[i].symbol, Some(pair));
                }
                age[i] = Some(a.saturating_add(1));
            }

            term.tick();

            let draw_beam = !path_done || !retract_done;
            if draw_beam {
                let beam = geometry::find_coords_on_line(emitter, laser_pos);
                frames.push(render_with_beam(&mut term, &beam, Some(laser_pos)));
            } else {
                frames.push(term.render_frame());
            }
        }

        if !cooled_or_shown(&age, &etchable) {
            for &idx in &etchable {
                if age[idx].is_none() {
                    term.set_character_visibility(infos[idx].id, true);
                }
                let pair = ColorPair::new(Some(finals[idx]), infos[idx].bg);
                if let Some(ch) = term.get_character_mut(infos[idx].id) {
                    ch.animation
                        .set_appearance(&infos[idx].symbol, Some(pair));
                }
            }
            frames.push(term.render_frame());
        }

        if frames.is_empty() {
            apply_finals(&mut term, &infos, &finals);
            for &idx in &etchable {
                term.set_character_visibility(infos[idx].id, true);
            }
            frames.push(term.render_frame());
        }

        frames
    }
}

const LASER_STEP: usize = 3;
const COOL_HOLD: usize = 2;
const HOLD_FRAMES: usize = 8;
const MAX_FRAMES: usize = 10_000;

struct EtchChar {
    id: CharacterId,
    coord: Coord,
    symbol: String,
    fg: Option<Color>,
    bg: Option<Color>,
}

fn hex(s: &str) -> Color {
    Color::from_hex(s).unwrap_or(Color::rgb(255, 255, 255))
}

fn is_etchable(info: &EtchChar) -> bool {
    info.symbol != " " || info.fg.is_some() || info.bg.is_some()
}

fn all_cooled(age: &[Option<usize>], cool_total: usize) -> bool {
    age.iter().all(|a| match *a {
        Some(v) => v >= cool_total,
        None => true,
    })
}

fn cooled_or_shown(age: &[Option<usize>], etchable: &[usize]) -> bool {
    etchable.iter().all(|&i| age[i].is_some())
}

fn vertical_color(
    coord: Coord,
    _min_col: i32,
    _max_col: i32,
    min_row: i32,
    max_row: i32,
    gradient: &Gradient,
) -> Option<Color> {
    let span = (max_row - min_row).max(1) as f64;
    let progress = (max_row - coord.row) as f64 / span;
    gradient.mapped_color(progress)
}

fn nearest_neighbor(infos: &[EtchChar], etchable: &[usize], start: Coord) -> Vec<usize> {
    let mut remaining = etchable.to_vec();
    let mut order = Vec::with_capacity(remaining.len());
    let mut current = start;
    while !remaining.is_empty() {
        let mut best = 0usize;
        let mut best_d = i64::MAX;
        for (ri, &idx) in remaining.iter().enumerate() {
            let c = infos[idx].coord;
            let dc = i64::from(c.column) - i64::from(current.column);
            let dr = i64::from(c.row) - i64::from(current.row);
            let d = dc * dc + dr * dr;
            if d < best_d {
                best_d = d;
                best = ri;
            }
        }
        let idx = remaining.remove(best);
        current = infos[idx].coord;
        order.push(idx);
    }
    order
}

fn apply_finals(term: &mut Terminal, infos: &[EtchChar], finals: &[Color]) {
    for (info, color) in infos.iter().zip(finals.iter()) {
        let pair = ColorPair::new(Some(*color), info.bg);
        if let Some(ch) = term.get_character_mut(info.id) {
            ch.animation.set_appearance(&info.symbol, Some(pair));
        }
    }
}

fn render_with_beam(term: &mut Terminal, beam: &[Coord], head: Option<Coord>) -> String {
    let cells: Vec<(Coord, CharacterVisual)> = term
        .get_characters()
        .iter()
        .filter(|ch| ch.is_visible)
        .map(|ch| {
            let mut visual = ch.animation.current_character_visual.clone();
            if visual.symbol.is_empty() {
                visual.symbol = ch.input_symbol.clone();
                visual.refresh();
            }
            (ch.current_coord(), visual)
        })
        .collect();

    let occupied: HashSet<Coord> = cells.iter().map(|(c, _)| *c).collect();

    term.canvas.clear();
    for (coord, visual) in cells {
        term.canvas.put(coord, visual);
    }
    for &coord in beam {
        if occupied.contains(&coord) || !term.canvas.contains(coord) {
            continue;
        }
        term.canvas.put(coord, CharacterVisual::new("·", None));
    }
    if let Some(h) = head {
        if !occupied.contains(&h) && term.canvas.contains(h) {
            term.canvas.put(h, CharacterVisual::new("*", None));
        }
    }
    term.canvas.render()
}
