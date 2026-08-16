use super::Effect;
use crate::engine::{Terminal, TerminalConfig};
use crate::utils::geometry::{distance, lerp_coord, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

const MOVEMENT_SPEED: f64 = 0.3;
const GRADIENT_STEPS: usize = 12;

fn in_out_quart(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        8.0 * t * t * t * t
    } else {
        let u = -2.0 * t + 2.0;
        1.0 - (u * u * u * u) / 2.0
    }
}

pub struct Expand;

impl Expand {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Expand {
    fn name(&self) -> &str {
        "expand"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        if term.character_count() == 0 {
            return vec![term.render_frame()];
        }

        let center = term.canvas.center();
        let input_coords: Vec<Coord> = term
            .get_characters()
            .iter()
            .map(|ch| ch.input_coord)
            .collect();

        let min_row = input_coords
            .iter()
            .map(|c| c.row)
            .min()
            .unwrap_or(center.row);
        let max_row = input_coords
            .iter()
            .map(|c| c.row)
            .max()
            .unwrap_or(center.row);
        let row_span = f64::from((max_row - min_row).max(1));

        let gradient = Gradient::new(
            &[
                Color::rgb(0x8A, 0x00, 0x8A),
                Color::rgb(0x00, 0xD1, 0xFF),
                Color::rgb(0xFF, 0xFF, 0xFF),
            ],
            GRADIENT_STEPS,
        );

        for ch in term.get_characters_mut() {
            let progress = f64::from(ch.input_coord.row - min_row) / row_span;
            let color = gradient
                .mapped_color(progress)
                .unwrap_or_else(|| Color::rgb(0xFF, 0xFF, 0xFF));
            let symbol = ch.input_symbol.clone();
            ch.animation
                .set_appearance(&symbol, Some(ColorPair::fg(color)));
            ch.motion.current_coord = center;
            ch.is_visible = true;
        }

        let mut elapsed = vec![0usize; input_coords.len()];
        let totals: Vec<usize> = input_coords
            .iter()
            .map(|&dest| {
                let d = distance(center, dest);
                if d == 0.0 {
                    0
                } else {
                    (d / MOVEMENT_SPEED).ceil() as usize
                }
            })
            .collect();

        let mut frames = Vec::new();
        loop {
            let mut moved = false;
            {
                let chars = term.get_characters_mut();
                for (i, ch) in chars.iter_mut().enumerate() {
                    if elapsed[i] < totals[i] {
                        elapsed[i] += 1;
                        let t = elapsed[i] as f64 / totals[i] as f64;
                        ch.motion.current_coord =
                            lerp_coord(center, input_coords[i], in_out_quart(t));
                        moved = true;
                    } else {
                        ch.motion.current_coord = input_coords[i];
                    }
                }
            }
            if !moved && !frames.is_empty() {
                break;
            }
            frames.push(term.render_frame());
            if !moved {
                break;
            }
        }
        frames
    }
}
