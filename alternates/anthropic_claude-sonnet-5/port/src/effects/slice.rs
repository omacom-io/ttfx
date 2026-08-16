use super::Effect;
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Slice: splits the canvas into left/right halves per row and slides each
/// half in from the top or bottom edge until it settles into its original
/// position, mirroring `terminaltexteffects/effects/effect_slice.py`'s
/// horizontal split (simplified to a single direction for this port).
pub struct Slice;

impl Slice {
    pub fn new() -> Self {
        Slice
    }
}

/// Per-character slide plan: where it starts (off to the top or bottom edge)
/// and where it must end up (its original input coordinate).
struct Plan {
    start: Coord,
    home: Coord,
}

impl Effect for Slice {
    fn name(&self) -> &str {
        "slice"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;
        let center_column = (width - 1) / 2;
        let top_row = 0;
        let bottom_row = height - 1;

        // Gradient used to paint the halves, standing in for the
        // fg_gradient/bg_gradient ColorPair scenes the Python effect builds
        // per character.
        let gradient = Gradient::new(
            &[
                Color::Rgb(30, 60, 200),
                Color::Rgb(90, 200, 255),
                Color::Rgb(255, 255, 255),
            ],
            12,
        );
        let spectrum_len = gradient.len().max(1);

        // Left half of each row slides down from the canvas top, right half
        // slides up from the canvas bottom, both converging on their
        // original `input_coord`, matching the left_half/right_half split
        // in effect_slice.py's horizontal branch.
        let mut plans: Vec<Plan> = Vec::with_capacity(terminal.get_characters().len());
        for character in terminal.get_characters() {
            let home = character.input_coord;
            let start = if home.column <= center_column {
                Coord::new(home.column, top_row)
            } else {
                Coord::new(home.column, bottom_row)
            };
            plans.push(Plan { start, home });
        }

        // Assign color (keyed off home column so the gradient reads
        // left-to-right across the settled text) and place every character
        // at its slide-in origin.
        {
            let characters = terminal.get_characters_mut();
            for (character, plan) in characters.iter_mut().zip(plans.iter()) {
                let color_index = (plan.home.column.max(0) as usize) % spectrum_len;
                let color = gradient.get(color_index).unwrap_or(Color::Rgb(255, 255, 255));
                character.animation.set_appearance(
                    character.input_symbol,
                    Some(ColorPair::new(Some(color), None)),
                );
                character.motion.current_coord = plan.start;
            }
        }

        let frame_count = (height.max(width).max(1) as usize) * 2 + 10;
        let mut frames = Vec::with_capacity(frame_count + 1);

        for frame_idx in 0..frame_count {
            let raw_t = (frame_idx + 1) as f64 / frame_count as f64;
            let t = raw_t.clamp(0.0, 1.0);
            let eased_t = easing::ease_in_out_quad(t);

            {
                let characters = terminal.get_characters_mut();
                for (character, plan) in characters.iter_mut().zip(plans.iter()) {
                    let start_col = plan.start.column as f64;
                    let start_row = plan.start.row as f64;
                    let home_col = plan.home.column as f64;
                    let home_row = plan.home.row as f64;
                    let column = start_col + (home_col - start_col) * eased_t;
                    let row = start_row + (home_row - start_row) * eased_t;
                    character.motion.current_coord =
                        Coord::new(column.round() as i32, row.round() as i32);
                }
            }

            frames.push(terminal.render());
        }

        // Guarantee a final settled frame with every character resting
        // exactly at its home coordinate.
        {
            let characters = terminal.get_characters_mut();
            for (character, plan) in characters.iter_mut().zip(plans.iter()) {
                character.motion.current_coord = plan.home;
            }
        }
        frames.push(terminal.render());

        frames
    }
}
