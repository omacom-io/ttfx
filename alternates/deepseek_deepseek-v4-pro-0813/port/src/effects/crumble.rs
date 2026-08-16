use crate::engine::canvas::{Canvas, Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use super::Effect;

pub struct Crumble;

impl Crumble {
    pub fn new() -> Self {
        Self
    }
}

fn put_cell(canvas: &mut Canvas, x: f32, y: f32, symbol: &str) {
    let x = x.round() as u16;
    let y = y.round() as u16;

    if x < canvas.width && y < canvas.height {
        canvas.set_cell(x, y, Cell::new(symbol, CellStyle::default()));
    }
}

impl Effect for Crumble {
    fn name(&self) -> &str {
        "crumble"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (mut width, mut height) = Terminal::autodetect_size();
        if width == 0 {
            width = 80;
        }
        if height == 0 {
            height = 24;
        }

        // Never clip the input; grow the canvas if necessary.
        let required_width = input
            .lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0)
            .max(1) as u16;
        let required_height = input.lines().count().max(1) as u16;
        width = width.max(required_width);
        height = height.max(required_height);

        let mut terminal = Terminal::from_input(input, width, height);

        let starts: Vec<(f32, f32)> = terminal
            .characters
            .iter()
            .map(|c| (c.position.x, c.position.y))
            .collect();
        let symbols: Vec<String> = terminal
            .characters
            .iter()
            .map(|c| c.output_symbol.clone())
            .collect();

        let mut frames = Vec::new();
        frames.push(terminal.write_frame());

        if starts.is_empty() {
            return frames;
        }

        let mut tick: u32 = 0;

        loop {
            terminal.clear_canvas();
            let mut any_active = false;

            for (idx, &(start_x, start_y)) in starts.iter().enumerate() {
                if symbols[idx].chars().all(|c| c.is_whitespace()) {
                    continue;
                }

                let start_frame = (start_y * 1.5) as u32 + (idx as u32 % 7);

                if tick < start_frame {
                    put_cell(&mut terminal.canvas, start_x, start_y, &symbols[idx]);
                    any_active = true;
                    continue;
                }

                let distance = (height as f32 - start_y).max(1.0);
                let fall_duration = (5.0 + distance * 0.6) as u32;
                let progress = (tick - start_frame) as f32 / fall_duration as f32;

                if progress < 1.0 {
                    let eased = easing::ease_in_quad(progress);
                    let y = start_y + distance * eased;
                    put_cell(&mut terminal.canvas, start_x, y, &symbols[idx]);
                    any_active = true;
                }
            }

            if !any_active {
                break;
            }

            frames.push(terminal.write_frame());
            tick += 1;

            // Safety valve; not expected to be reached in normal use.
            if tick > 2000 {
                break;
            }
        }

        frames
    }
}
