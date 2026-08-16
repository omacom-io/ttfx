use super::Effect;
use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, Gradient};

pub struct Sweep;

impl Sweep {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Sweep {
    fn name(&self) -> &str {
        "sweep"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (width, height) = Terminal::autodetect_size();
        let mut terminal = Terminal::from_input(input, width, height);

        let max_x = terminal
            .characters
            .iter()
            .map(|c| c.position.x as i32)
            .max()
            .unwrap_or(0);

        let sweep_gradient = Gradient::new()
            .add_stop(0.0, Color::CYAN)
            .add_stop(0.5, Color::MAGENTA)
            .add_stop(1.0, Color::WHITE);

        let characters = terminal.characters.clone();
        let mut frames = Vec::new();

        // Sweep a reveal edge from just off the left edge to beyond the last character.
        for leading_edge in -4..=max_x + 6 {
            terminal.clear_canvas();

            for character in &characters {
                let x = character.position.x.round() as i32;
                let y = character.position.y.round() as i32;

                if x > leading_edge {
                    continue;
                }

                if x < 0 || x >= width as i32 || y < 0 || y >= height as i32 {
                    continue;
                }

                let gradient_position = ((leading_edge - x) as f32 / 4.0).clamp(0.0, 1.0);
                let fg = sweep_gradient.color_at(gradient_position);
                let style = CellStyle::new(fg, Color::BLACK);
                let cell = Cell::new(character.input_symbol.clone(), style);

                terminal.canvas.set_cell(x as u16, y as u16, cell);
            }

            frames.push(terminal.write_frame());
        }

        frames
    }
}
