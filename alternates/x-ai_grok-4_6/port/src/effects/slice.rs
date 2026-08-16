use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{find_length_of_line, lerp_coord, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

const MOVEMENT_SPEED: f64 = 0.5;
const GRADIENT_STEPS: usize = 12;

pub struct Slice;

impl Slice {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Slice {
    fn default() -> Self {
        Self::new()
    }
}

/// Python `easing.in_out_quad`.
fn in_out_quad(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    if progress < 0.5 {
        2.0 * progress * progress
    } else {
        (-2.0 * progress * progress) + (4.0 * progress) - 1.0
    }
}

struct SliceMove {
    index: usize,
    start: Coord,
    end: Coord,
    steps: usize,
}

impl Effect for Slice {
    fn name(&self) -> &str {
        "slice"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_input(input, TerminalConfig::default());
        if terminal.character_count() == 0 {
            return vec![terminal.render_frame()];
        }

        let (text_left, text_right, text_bottom, text_top) = {
            let chars = terminal.get_characters();
            let mut left = chars[0].input_coord.column;
            let mut right = left;
            let mut bottom = chars[0].input_coord.row;
            let mut top = bottom;
            for ch in chars.iter().skip(1) {
                left = left.min(ch.input_coord.column);
                right = right.max(ch.input_coord.column);
                bottom = bottom.min(ch.input_coord.row);
                top = top.max(ch.input_coord.row);
            }
            (left, right, bottom, top)
        };

        // Python: text_left + ((text_right - text_left) // 2)
        let text_center_column = text_left + (text_right - text_left) / 2;
        let canvas_top = terminal.canvas.top;
        let canvas_bottom = terminal.canvas.bottom;
        let row_span = (text_top - text_bottom).max(1) as f64;

        let stops = [
            Color::from_hex("8A008A").unwrap_or(Color::rgb(0x8A, 0x00, 0x8A)),
            Color::from_hex("00D1FF").unwrap_or(Color::rgb(0x00, 0xD1, 0xFF)),
            Color::from_hex("FFFFFF").unwrap_or(Color::rgb(0xFF, 0xFF, 0xFF)),
        ];
        let gradient = Gradient::new(&stops, GRADIENT_STEPS);

        let mut moves = Vec::new();
        {
            let characters = terminal.get_characters_mut();
            for (index, ch) in characters.iter_mut().enumerate() {
                let end = ch.input_coord;
                let progress = (end.row - text_bottom) as f64 / row_span;
                if let Some(color) = gradient.mapped_color(progress) {
                    let symbol = ch.input_symbol.clone();
                    ch.animation
                        .set_appearance(&symbol, Some(ColorPair::fg(color)));
                }

                // vertical (default): left half drops in from above, right half rises from below
                let start = if end.column <= text_center_column {
                    Coord::new(end.column, canvas_top + 1)
                } else {
                    Coord::new(end.column, canvas_bottom - 1)
                };

                let dist = find_length_of_line(start, end);
                let steps = if dist <= 0.0 {
                    1
                } else {
                    (dist / MOVEMENT_SPEED).ceil().max(1.0) as usize
                };

                ch.motion.current_coord = start;
                ch.is_visible = true;
                moves.push(SliceMove {
                    index,
                    start,
                    end,
                    steps,
                });
            }
        }

        let total = moves.iter().map(|m| m.steps).max().unwrap_or(1);
        let mut frames = Vec::with_capacity(total);
        for step in 1..=total {
            {
                let characters = terminal.get_characters_mut();
                for m in &moves {
                    let t = (step as f64 / m.steps as f64).min(1.0);
                    characters[m.index].motion.current_coord =
                        lerp_coord(m.start, m.end, in_out_quad(t));
                    characters[m.index].is_visible = true;
                }
            }
            frames.push(terminal.render_frame());
        }
        frames
    }
}
