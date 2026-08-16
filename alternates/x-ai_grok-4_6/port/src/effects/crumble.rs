use std::collections::HashMap;

use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::motion::Motion;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{find_coord_on_bezier_curve, lerp_coord, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

const HIGHLIGHT: Color = Color { r: 255, g: 255, b: 255 };
const DUST: Color = Color { r: 0, g: 0, b: 0 };
const FLASH_FRAMES: u32 = 12;
const WEAKEN_FRAMES: u32 = 16;
const INTRO_HOLD: usize = 8;
const DUST_HOLD: usize = 6;
const END_HOLD: usize = 8;
const ACTIVATE_PER_FRAME: usize = 3;
const MAX_FRAMES: usize = 20_000;

pub struct Crumble;

impl Crumble {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Crumble {
    fn name(&self) -> &str {
        "crumble"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        let bottom = term.canvas.bottom;
        let top = term.canvas.top;
        let center = term.canvas.center();

        let infos = collect_infos(&term);
        if infos.is_empty() {
            term.show_all();
            return vec![term.render_frame()];
        }

        term.show_all();
        for info in &infos {
            paint(&mut term, info, info.input, info.final_color);
        }

        let mut phases: HashMap<CharacterId, Phase> = infos
            .iter()
            .map(|info| (info.id, Phase::Idle))
            .collect();
        let mut frames: Vec<String> = Vec::new();

        for _ in 0..INTRO_HOLD {
            frames.push(term.render_frame());
        }

        while frames.len() < MAX_FRAMES {
            activate_unsupported(&infos, &mut phases);
            apply_all(&mut term, &infos, &phases, bottom, top, center);
            frames.push(term.render_frame());
            step_all(&infos, &mut phases, bottom);
            if phases.values().all(|phase| matches!(phase, Phase::Fallen)) {
                break;
            }
        }

        for _ in 0..DUST_HOLD {
            if frames.len() >= MAX_FRAMES {
                break;
            }
            apply_all(&mut term, &infos, &phases, bottom, top, center);
            frames.push(term.render_frame());
        }

        for info in &infos {
            phases.insert(info.id, Phase::Fly { frame: 0 });
        }
        while frames.len() < MAX_FRAMES {
            apply_all(&mut term, &infos, &phases, bottom, top, center);
            frames.push(term.render_frame());
            step_all(&infos, &mut phases, bottom);
            if phases.values().all(|phase| matches!(phase, Phase::Flown)) {
                break;
            }
        }

        for info in &infos {
            phases.insert(info.id, Phase::Return { frame: 0 });
        }
        while frames.len() < MAX_FRAMES {
            apply_all(&mut term, &infos, &phases, bottom, top, center);
            frames.push(term.render_frame());
            step_all(&infos, &mut phases, bottom);
            if phases.values().all(|phase| matches!(phase, Phase::Done)) {
                break;
            }
        }

        for _ in 0..END_HOLD {
            if frames.len() >= MAX_FRAMES {
                break;
            }
            apply_all(&mut term, &infos, &phases, bottom, top, center);
            frames.push(term.render_frame());
        }

        frames
    }
}

#[derive(Clone, Copy)]
enum Phase {
    Idle,
    Flash { frame: u32 },
    Weaken { frame: u32 },
    Fall { frame: u32 },
    Fallen,
    Fly { frame: u32 },
    Flown,
    Return { frame: u32 },
    Done,
}

struct CharInfo {
    id: CharacterId,
    symbol: String,
    input: Coord,
    above: Option<CharacterId>,
    final_color: Color,
}

fn collect_infos(term: &Terminal) -> Vec<CharInfo> {
    let chars = term.get_characters();
    let min_row = chars.iter().map(|ch| ch.input_coord.row).min().unwrap_or(1);
    let max_row = chars.iter().map(|ch| ch.input_coord.row).max().unwrap_or(1);
    let stops = [
        Color::from_hex("bb97ff").unwrap_or(Color::rgb(187, 151, 255)),
        Color::from_hex("8c82fc").unwrap_or(Color::rgb(140, 130, 252)),
        Color::from_hex("675af6").unwrap_or(Color::rgb(103, 90, 246)),
    ];
    let gradient = Gradient::new(&stops, 12);
    let span = f64::from((max_row - min_row).max(1));
    chars
        .iter()
        .map(|ch| {
            let progress = f64::from(ch.input_coord.row - min_row) / span;
            let final_color = gradient.mapped_color(progress).unwrap_or(stops[0]);
            CharInfo {
                id: ch.id,
                symbol: ch.input_symbol.clone(),
                input: ch.input_coord,
                above: ch.neighbors.above,
                final_color,
            }
        })
        .collect()
}

fn activate_unsupported(infos: &[CharInfo], phases: &mut HashMap<CharacterId, Phase>) {
    let mut candidates: Vec<(i32, i32, CharacterId)> = Vec::new();
    for info in infos {
        if !matches!(phases.get(&info.id), Some(Phase::Idle)) {
            continue;
        }
        let unsupported = match info.above {
            None => true,
            Some(above) => matches!(
                phases.get(&above),
                Some(Phase::Fall { .. } | Phase::Fallen | Phase::Fly { .. } | Phase::Flown | Phase::Return { .. } | Phase::Done)
            ),
        };
        if unsupported {
            candidates.push((-info.input.row, info.input.column, info.id));
        }
    }
    candidates.sort_unstable();
    for (_, _, id) in candidates.into_iter().take(ACTIVATE_PER_FRAME) {
        phases.insert(id, Phase::Flash { frame: 0 });
    }
}

fn step_all(infos: &[CharInfo], phases: &mut HashMap<CharacterId, Phase>, bottom: i32) {
    for info in infos {
        let Some(phase) = phases.get_mut(&info.id) else {
            continue;
        };
        match phase {
            Phase::Idle | Phase::Fallen | Phase::Flown | Phase::Done => {}
            Phase::Flash { frame } => {
                *frame += 1;
                if *frame >= FLASH_FRAMES {
                    *phase = Phase::Weaken { frame: 0 };
                }
            }
            Phase::Weaken { frame } => {
                *frame += 1;
                if *frame >= WEAKEN_FRAMES {
                    *phase = Phase::Fall { frame: 0 };
                }
            }
            Phase::Fall { frame } => {
                *frame += 1;
                if *frame >= fall_duration(info.input.row, bottom) {
                    *phase = Phase::Fallen;
                }
            }
            Phase::Fly { frame } => {
                *frame += 1;
                if *frame >= fly_duration(info.input.row, bottom) {
                    *phase = Phase::Flown;
                }
            }
            Phase::Return { frame } => {
                *frame += 1;
                if *frame >= return_duration(info.input.row, bottom) {
                    *phase = Phase::Done;
                }
            }
        }
    }
}

fn apply_all(
    term: &mut Terminal,
    infos: &[CharInfo],
    phases: &HashMap<CharacterId, Phase>,
    bottom: i32,
    top: i32,
    center: Coord,
) {
    for info in infos {
        let phase = phases.get(&info.id).copied().unwrap_or(Phase::Idle);
        let (coord, color) = visual_for(info, phase, bottom, top, center);
        paint(term, info, coord, color);
    }
}

fn visual_for(info: &CharInfo, phase: Phase, bottom: i32, top: i32, center: Coord) -> (Coord, Color) {
    match phase {
        Phase::Idle => (info.input, info.final_color),
        Phase::Flash { frame } => {
            let gradient = Gradient::new(&[info.final_color, HIGHLIGHT], 6);
            let color = gradient
                .mapped_color(progress(frame, FLASH_FRAMES))
                .unwrap_or(HIGHLIGHT);
            (info.input, color)
        }
        Phase::Weaken { frame } => {
            let gradient = Gradient::new(&[HIGHLIGHT, DUST], 6);
            let color = gradient
                .mapped_color(progress(frame, WEAKEN_FRAMES))
                .unwrap_or(DUST);
            (info.input, color)
        }
        Phase::Fall { frame } => {
            let dest = Coord::new(info.input.column, bottom);
            let t = out_bounce(progress(frame, fall_duration(info.input.row, bottom)));
            (lerp_coord(info.input, dest, t), DUST)
        }
        Phase::Fallen => (Coord::new(info.input.column, bottom), DUST),
        Phase::Fly { frame } => {
            let start = Coord::new(info.input.column, bottom);
            let dest = Coord::new(info.input.column, top);
            let t = out_quint(progress(frame, fly_duration(info.input.row, bottom)));
            (find_coord_on_bezier_curve(start, center, dest, t), DUST)
        }
        Phase::Flown => (Coord::new(info.input.column, top), DUST),
        Phase::Return { frame } => {
            let start = Coord::new(info.input.column, top);
            let t = out_quint(progress(frame, return_duration(info.input.row, bottom)));
            let gradient = Gradient::new(&[DUST, info.final_color], 6);
            let color = gradient.mapped_color(t).unwrap_or(info.final_color);
            (lerp_coord(start, info.input, t), color)
        }
        Phase::Done => (info.input, info.final_color),
    }
}

fn paint(term: &mut Terminal, info: &CharInfo, coord: Coord, color: Color) {
    if let Some(ch) = term.get_character_mut(info.id) {
        ch.motion = Motion::new(coord);
        ch.is_visible = true;
        ch.animation
            .set_appearance(&info.symbol, Some(ColorPair::fg(color)));
    }
}

fn fall_duration(from_row: i32, bottom: i32) -> u32 {
    (from_row - bottom).unsigned_abs().max(1).saturating_mul(3).clamp(18, 48)
}

fn fly_duration(from_row: i32, bottom: i32) -> u32 {
    (from_row - bottom).unsigned_abs().max(1).saturating_mul(2).clamp(16, 36)
}

fn return_duration(from_row: i32, bottom: i32) -> u32 {
    (from_row - bottom).unsigned_abs().max(1).saturating_mul(2).clamp(18, 40)
}

fn progress(frame: u32, total: u32) -> f64 {
    if total <= 1 {
        1.0
    } else {
        (f64::from(frame) / f64::from(total - 1)).clamp(0.0, 1.0)
    }
}

fn out_bounce(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    let n1 = 7.5625;
    let d1 = 2.75;
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

fn out_quint(t: f64) -> f64 {
    let u = 1.0 - t.clamp(0.0, 1.0);
    1.0 - u * u * u * u * u
}
