use std::collections::BTreeMap;

use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{self, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

const MOVEMENT_SPEED: f64 = 0.5;
const GAP: i32 = 3;
const GRADIENT_STEPS: usize = 12;
const MAX_FRAMES: usize = 10_000;

/// Python `easing.in_out_quad`.
fn in_out_quad(progress: f64) -> f64 {
    if progress < 0.5 {
        2.0 * progress * progress
    } else {
        let t = -2.0 * progress + 2.0;
        1.0 - (t * t) / 2.0
    }
}

struct Mover {
    id: CharacterId,
    start: Coord,
    end: Coord,
    increment: f64,
    progress: f64,
    started: bool,
}

impl Mover {
    fn is_moving(&self) -> bool {
        self.started && self.progress < 1.0
    }
}

pub struct Slide;

impl Slide {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Slide {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Slide {
    fn name(&self) -> &str {
        "slide"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_input(input, TerminalConfig::default());
        if terminal.character_count() == 0 {
            return vec![terminal.render_frame()];
        }

        let text_bottom = terminal
            .get_characters()
            .iter()
            .map(|ch| ch.input_coord.row)
            .min()
            .unwrap_or(terminal.canvas.bottom);
        let text_top = terminal
            .get_characters()
            .iter()
            .map(|ch| ch.input_coord.row)
            .max()
            .unwrap_or(terminal.canvas.top);

        let stops = [
            Color::from_hex("8A008A").unwrap_or(Color::rgb(0x8A, 0x00, 0x8A)),
            Color::from_hex("00D1FF").unwrap_or(Color::rgb(0x00, 0xD1, 0xFF)),
            Color::from_hex("FFFFFF").unwrap_or(Color::rgb(0xFF, 0xFF, 0xFF)),
        ];
        let gradient = Gradient::new(&stops, GRADIENT_STEPS);
        let row_span = f64::from((text_top - text_bottom).max(1));

        // Default grouping is row, no merge, no reverse: each row starts just
        // off the right edge and is walked right-to-left.
        let start_column = terminal.canvas.right + 1;
        let mut movers: Vec<Mover> = Vec::with_capacity(terminal.character_count());

        for ch in terminal.get_characters_mut() {
            let progress = f64::from(ch.input_coord.row - text_bottom) / row_span;
            let color = gradient
                .mapped_color(progress.clamp(0.0, 1.0))
                .unwrap_or(stops[stops.len() - 1]);
            let symbol = ch.input_symbol.clone();
            ch.animation
                .set_appearance(&symbol, Some(ColorPair::fg(color)));

            let start = Coord::new(start_column, ch.input_coord.row);
            let end = ch.input_coord;
            ch.motion.current_coord = start;
            ch.is_visible = false;

            let distance = geometry::find_length_of_line(start, end);
            movers.push(Mover {
                id: ch.id,
                start,
                end,
                increment: if distance <= f64::EPSILON {
                    1.0
                } else {
                    MOVEMENT_SPEED / distance
                },
                progress: 0.0,
                started: false,
            });
        }

        // ROW_TOP_TO_BOTTOM, then reverse each group (default, no merge / reverse).
        let mut rows: BTreeMap<i32, Vec<CharacterId>> = BTreeMap::new();
        for ch in terminal.get_characters() {
            rows.entry(ch.input_coord.row).or_default().push(ch.id);
        }
        let mut pending_groups: Vec<Vec<CharacterId>> = rows
            .into_iter()
            .rev()
            .map(|(_, mut group)| {
                group.reverse();
                group
            })
            .filter(|group| !group.is_empty())
            .collect();

        let mut frames = Vec::new();
        let mut gap = 0_i32;

        while (!pending_groups.is_empty() || movers.iter().any(Mover::is_moving))
            && frames.len() < MAX_FRAMES
        {
            if gap == 0 {
                if !pending_groups.is_empty() {
                    let group = pending_groups.remove(0);
                    for id in group {
                        if let Some(mover) = movers.iter_mut().find(|m| m.id == id) {
                            mover.started = true;
                            mover.progress = 0.0;
                            if let Some(ch) = terminal.get_character_mut(id) {
                                ch.motion.current_coord = mover.start;
                                ch.is_visible = true;
                            }
                        }
                    }
                    gap = GAP;
                }
            } else {
                gap -= 1;
            }

            for mover in movers.iter_mut() {
                if !mover.is_moving() {
                    continue;
                }
                mover.progress = (mover.progress + mover.increment).min(1.0);
                let eased = in_out_quad(mover.progress);
                let coord = geometry::lerp_coord(mover.start, mover.end, eased);
                if let Some(ch) = terminal.get_character_mut(mover.id) {
                    ch.motion.current_coord = coord;
                }
            }

            frames.push(terminal.render_frame());
        }

        if frames.is_empty() {
            terminal.show_all();
            frames.push(terminal.render_frame());
        }

        frames
    }
}
