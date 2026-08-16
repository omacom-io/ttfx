use super::Effect;
use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::easing::ease_out_quint;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Gradient};

pub struct Pour;

impl Pour {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Pour {
    fn name(&self) -> &str {
        "pour"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        const FRAMES: usize = 30;

        let (width, height) = Terminal::autodetect_size();
        let mut terminal = Terminal::from_input(input, width, height);

        let final_positions: Vec<Coord> = terminal.characters.iter().map(|c| c.position).collect();

        let gradient = Gradient::new()
            .add_stop(0.0, Color::new(0, 191, 255))
            .add_stop(1.0, Color::WHITE);

        let mut frames = Vec::with_capacity(FRAMES + 1);

        for frame_index in 0..=FRAMES {
            let global_t = frame_index as f32 / FRAMES as f32;

            terminal.canvas.clear();

            for (index, character) in terminal.characters.iter_mut().enumerate() {
                let final_pos = final_positions[index];

                let delay = (final_pos.x as f32 / width as f32) * 0.2
                    + (final_pos.y as f32 / height as f32) * 0.1;
                let local_t = ((global_t - delay) / (1.0 - delay)).clamp(0.0, 1.0);
                let eased = ease_out_quint(local_t);

                let start_y = 0.0f32;
                let current_y = start_y + (final_pos.y - start_y) * eased;
                character.position = Coord::new(final_pos.x, current_y);

                let color = gradient.color_at(local_t);
                character.style = CellStyle::new(color, Color::BLACK);

                if current_y >= 0.0 && current_y < height as f32 {
                    let x = final_pos.x.round() as u16;
                    let y = current_y.round() as u16;
                    if x < width {
                        terminal.canvas.set_cell(
                            x,
                            y,
                            Cell::new(character.input_symbol.clone(), character.style),
                        );
                    }
                }
            }

            frames.push(terminal.write_frame());
        }

        frames
    }
}
