use super::Effect;
use crate::engine::canvas::{Canvas, Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::geometry::Coord;
use crate::utils::graphics::Color;

pub struct Matrix;

impl Matrix {
    pub fn new() -> Self {
        Self
    }
}

const MATRIX_TRAIL: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
    'A', 'B', 'C', 'D', 'E', 'F',
    '!', '#', '$', '%', '&', '*', '?',
    'a', 'b', 'c', 'd', 'e', 'f',
];

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn range(&mut self, min: usize, max: usize) -> usize {
        if max <= min {
            return min;
        }
        min + (self.next_u64() as usize % (max - min))
    }
}

impl Effect for Matrix {
    fn name(&self) -> &str {
        "matrix"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (mut width, mut height) = Terminal::autodetect_size();
        if width == 0 {
            width = 80;
        }
        if height == 0 {
            height = 24;
        }

        let terminal = Terminal::from_input(input, width, height);
        let input_chars: Vec<(u32, String, Coord)> = terminal
            .characters
            .iter()
            .map(|c| (c.id, c.input_symbol.clone(), c.position))
            .collect();

        if input_chars.is_empty() {
            return vec![terminal.write_frame()];
        }

        let fall_duration = height as usize + 6;
        let max_delay = ((height as usize / 4).max(1)).min((width as usize / 2).max(1));
        let hold_frames = 8;
        let total_frames = fall_duration + max_delay + hold_frames;

        let mut rng = Rng::new(0x9e3779b97f4a7c15);
        let mut frames = Vec::with_capacity(total_frames);

        for frame_index in 0..total_frames {
            let mut canvas = Canvas::new(width, height);

            for (character_id, symbol, position) in &input_chars {
                let column = position.x.round() as u16;
                let target_row = position.y.round() as u16;

                let start_delay = (column as usize * 37 + *character_id as usize) % max_delay;
                if frame_index < start_delay {
                    continue;
                }

                let local_frame = frame_index - start_delay;
                let progress = local_frame as f32 / fall_duration as f32;

                let head_row = if progress >= 1.0 {
                    target_row
                } else {
                    (progress * target_row as f32).round() as u16
                };

                let head_style = CellStyle::new(Color::WHITE, Color::BLACK);
                canvas.set_cell(column, head_row, Cell::new(symbol.clone(), head_style));

                if progress < 1.0 {
                    let trail_length = if height > 12 { 5 } else { 3 };

                    for trail_index in 1..=trail_length {
                        let tail_y = head_row as i32 - trail_index as i32;
                        if tail_y < 0 {
                            break;
                        }

                        let tail_y = tail_y as u16;
                        let tail_symbol = MATRIX_TRAIL[rng.range(0, MATRIX_TRAIL.len())];
                        let trail_style = CellStyle::new(Color::GREEN, Color::BLACK);

                        canvas.set_cell(
                            column,
                            tail_y,
                            Cell::new(tail_symbol.to_string(), trail_style),
                        );
                    }
                }
            }

            frames.push(canvas.render_frame());
        }

        frames
    }
}
