use super::Effect;
use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, Gradient};

pub struct Waves;

impl Waves {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Waves {
    fn name(&self) -> &str {
        "waves"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        if input.is_empty() {
            return Vec::new();
        }

        let line_count = input.lines().count().max(1) as u16;
        let width = input
            .lines()
            .map(|line| line.chars().count() as u16)
            .max()
            .unwrap_or(1)
            .max(1);

        let mut terminal = Terminal::from_input(input, width, line_count);

        let characters: Vec<(u16, u16, String)> = terminal
            .characters
            .iter()
            .map(|c| {
                (
                    c.position.x as u16,
                    c.position.y as u16,
                    c.input_symbol.clone(),
                )
            })
            .collect();

        let gradient = Gradient::new()
            .add_stop(0.0, Color::new(30, 144, 255))
            .add_stop(0.3, Color::new(0, 206, 209))
            .add_stop(0.7, Color::new(173, 216, 230))
            .add_stop(1.0, Color::WHITE);

        let frame_count = 60;
        let mut frames = Vec::with_capacity(frame_count);

        for frame_index in 0..frame_count {
            let t = frame_index as f32 * 0.5;

            for (x, y, symbol) in &characters {
                let phase = (*x as f32 * 0.35 + *y as f32 * 0.25 + t).sin();
                let mix = (phase + 1.0) * 0.5;
                let fg = gradient.color_at(mix);
                let style = CellStyle::new(fg, Color::BLACK);

                terminal
                    .canvas
                    .set_cell(*x, *y, Cell::new(symbol.as_str(), style));
            }

            frames.push(terminal.write_frame());
        }

        frames
    }
}
