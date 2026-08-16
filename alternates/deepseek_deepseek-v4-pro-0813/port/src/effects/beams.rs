
use super::Effect;
use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::graphics::{Color, Gradient};

pub struct Beams {
    width: u16,
    height: u16,
}

impl Beams {
    pub fn new() -> Self {
        let (width, height) = Terminal::autodetect_size();
        Self { width, height }
    }

    fn draw_vertical_beam(&self, terminal: &mut Terminal, beam_center: u16, beam_width: u16) {
        if self.width == 0 {
            return;
        }

        let half = beam_width / 2;
        let start = beam_center.saturating_sub(half);
        let end = beam_center.saturating_add(half);

        for x in start..=end {
            if x >= self.width {
                continue;
            }

            let distance = if x > beam_center {
                (x - beam_center) as f32
            } else {
                (beam_center - x) as f32
            };
            let intensity = (1.0 - distance / (beam_width as f32).max(1.0)).clamp(0.35, 1.0);

            let fg = Color::new(
                (180.0 * intensity) as u8,
                (220.0 * intensity) as u8,
                255,
            );
            let style = CellStyle::new(fg, Color::BLACK);

            for y in 0..self.height {
                terminal.canvas.set_cell(x, y, Cell::new("█", style));
            }
        }
    }
}

impl Effect for Beams {
    fn name(&self) -> &str {
        "beams"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_input(input, self.width, self.height);

        let beam_width: u16 = 7;
        let half = beam_width / 2;
        let steps = self.width / 2 + beam_width + 5;

        let color_gradient = Gradient::new()
            .add_stop(0.0, Color::CYAN)
            .add_stop(0.45, Color::MAGENTA)
            .add_stop(1.0, Color::WHITE);

        let mut final_styles: Vec<CellStyle> = Vec::with_capacity(terminal.characters.len());
        {
            let max_x = self.width.max(1) as f32;
            let max_y = self.height.max(1) as f32;

            for ch in &terminal.characters {
                let t = ((ch.position.x / max_x) * 0.65 + (ch.position.y / max_y) * 0.35)
                    .clamp(0.0, 1.0);
                final_styles.push(CellStyle::new(color_gradient.color_at(t), Color::BLACK));
            }
        }

        let mut frames = Vec::new();

        for step in 0..=steps {
            let progress = step as f32 / steps as f32;
            let eased = easing::ease_in_out_cubic(progress);

            let travel = self.width as f32 + beam_width as f32;

            let left_center = ((eased * travel).round() as i32 - half as i32).max(0) as u16;
            let right_center = ((self.width as f32 + half as f32) - eased * travel)
                .round()
                .max(0.0) as u16;

            terminal.clear_canvas();

            let left_edge = left_center.saturating_add(half);
            let right_edge = right_center.saturating_sub(half);

            for (i, ch) in terminal.characters.iter().enumerate() {
                let x = ch.position.x.round() as u16;
                let y = ch.position.y.round() as u16;

                if x <= left_edge || x >= right_edge {
                    terminal
                        .canvas
                        .set_cell(x, y, Cell::new(ch.input_symbol.clone(), final_styles[i]));
                }
            }

            self.draw_vertical_beam(&mut terminal, left_center, beam_width);
            self.draw_vertical_beam(&mut terminal, right_center, beam_width);

            frames.push(terminal.write_frame());
        }

        terminal.clear_canvas();
        for (i, ch) in terminal.characters.iter().enumerate() {
            let x = ch.position.x.round() as u16;
            let y = ch.position.y.round() as u16;
            terminal
                .canvas
                .set_cell(x, y, Cell::new(ch.input_symbol.clone(), final_styles[i]));
        }
        frames.push(terminal.write_frame());

        frames
    }
}
