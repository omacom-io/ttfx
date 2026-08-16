use super::Effect;
use crate::engine::canvas::{Canvas, Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::graphics::{Color, Gradient};

pub struct Overflow;

impl Overflow {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Overflow {
    fn name(&self) -> &str {
        "overflow"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (width, height) = Terminal::autodetect_size();
        let terminal = Terminal::from_input(input, width, height);
        let characters = terminal.characters.clone();

        if characters.is_empty() {
            return vec![terminal.write_frame()];
        }

        let total_frames = (height as usize * 4).max(30).min(90);
        let mut frames = Vec::with_capacity(total_frames);

        // A simple spill gradient: the character colours shift as they settle.
        let gradient = Gradient::new()
            .add_stop(0.0, Color::CYAN)
            .add_stop(0.55, Color::MAGENTA)
            .add_stop(1.0, Color::YELLOW);

        for frame_idx in 0..total_frames {
            let mut canvas = Canvas::new(width, height);
            let progress = frame_idx as f32 / (total_frames - 1) as f32;

            for character in &characters {
                let x = character.position.x;
                let final_y = character.position.y;

                // Spill from the top edge to the character's input row, with a
                // slight left-to-right stagger so the overflow reads as a wave.
                let width_for_delay = width.max(1) as f32;
                let delay = (x / width_for_delay) * 0.25;
                let local_t = ((progress - delay) / (1.0 - delay)).clamp(0.0, 1.0);
                let eased_t = easing::ease_out_bounce(local_t);
                let y = eased_t * final_y;

                let color = gradient.color_at(local_t);
                let style = CellStyle::new(color, Color::BLACK);
                canvas.set_cell(
                    x.round() as u16,
                    y.round() as u16,
                    Cell::new(character.input_symbol.clone(), style),
                );
            }

            frames.push(canvas.render_frame());
        }

        frames
    }
}
