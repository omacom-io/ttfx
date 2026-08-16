use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, Gradient};
use super::Effect;

pub struct Colorshift {
    gradient: Gradient,
    speed: f32,
}

impl Colorshift {
    pub fn new() -> Self {
        let gradient = Gradient::new()
            .add_stop(0.0, Color::RED)
            .add_stop(0.17, Color::YELLOW)
            .add_stop(0.33, Color::GREEN)
            .add_stop(0.50, Color::CYAN)
            .add_stop(0.67, Color::BLUE)
            .add_stop(0.83, Color::MAGENTA)
            .add_stop(1.0, Color::RED);

        Self {
            gradient,
            speed: 0.025,
        }
    }
}

impl Default for Colorshift {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Colorshift {
    fn name(&self) -> &str {
        "colorshift"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (width, height) = Terminal::autodetect_size();
        let mut terminal = Terminal::from_input(input, width, height);

        if terminal.characters.is_empty() {
            return vec![terminal.write_frame()];
        }

        let frame_count = 60usize;
        let mut frames = Vec::with_capacity(frame_count);

        for frame_index in 0..frame_count {
            let phase = frame_index as f32 * self.speed;

            for i in 0..terminal.characters.len() {
                let (x, y, symbol) = {
                    let character = &terminal.characters[i];
                    (
                        character.position.x,
                        character.position.y,
                        character.input_symbol.clone(),
                    )
                };

                let t = ((x + y) * 0.04 + phase).rem_euclid(1.0);
                let color = self.gradient.color_at(t);
                let style = CellStyle::new(color, Color::BLACK);
                let cell = Cell::new(symbol, style);

                terminal.canvas.set_cell(x.round() as u16, y.round() as u16, cell);
            }

            frames.push(terminal.write_frame());
        }

        frames
    }
}
