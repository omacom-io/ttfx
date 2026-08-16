use crate::engine::canvas::CellStyle;
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, Gradient};

use super::Effect;

pub struct Burn;

impl Burn {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Burn {
    fn name(&self) -> &str {
        "burn"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (width, height) = Terminal::autodetect_size();
        let mut terminal = Terminal::from_input(input, width, height);

        let background = terminal.config.default_style.bg;

        let character_cells: Vec<(u16, u16, u32, String)> = terminal
            .characters
            .iter()
            .map(|c| {
                (
                    c.position.x as u16,
                    c.position.y as u16,
                    c.id,
                    c.input_symbol.clone(),
                )
            })
            .collect();

        let frame_count = (height as usize * 4).clamp(40, 120);
        let mut frames = Vec::with_capacity(frame_count);

        let start_y = height as f32 + 5.0;
        let end_y = -5.0;

        let flame_gradient = Gradient::new()
            .add_stop(0.0, Color::new(180, 40, 15))
            .add_stop(0.35, Color::new(255, 100, 0))
            .add_stop(0.65, Color::new(255, 200, 70))
            .add_stop(0.9, Color::new(255, 245, 210))
            .add_stop(1.0, Color::new(35, 20, 12));

        let normal_fg = Color::new(210, 210, 210);
        let consumed_fg = Color::new(35, 25, 20);

        for frame_index in 0..frame_count {
            let t = frame_index as f32 / (frame_count - 1) as f32;
            let front_y = start_y - t * (start_y - end_y);

            for (x, y, id, symbol) in &character_cells {
                let row = *y as f32;
                let dist = row - front_y;

                let (cell_symbol, style) = if dist < 0.0 {
                    (symbol.as_str(), CellStyle::new(normal_fg, background))
                } else if dist <= 3.0 {
                    let flicker = flicker_noise(*id, frame_index);
                    let flame_pos = ((dist / 3.0) + flicker * 0.12 - 0.06).clamp(0.0, 1.0);
                    let fg = flame_gradient.color_at(flame_pos);
                    let mut style = CellStyle::new(fg, background);
                    if flame_pos < 0.7 {
                        style.bold = true;
                    }
                    (symbol.as_str(), style)
                } else {
                    (" ", CellStyle::new(consumed_fg, background))
                };

                if let Some(cell) = terminal.canvas.get_mut(*x, *y) {
                    cell.symbol = cell_symbol.to_string();
                    cell.style = style;
                }
            }

            frames.push(terminal.write_frame());
        }

        frames
    }
}

fn flicker_noise(id: u32, frame_index: usize) -> f32 {
    let mut v = id
        .wrapping_mul(1_103_515_245)
        .wrapping_add(frame_index as u32 * 12_345);
    v ^= v >> 11;
    ((v >> 8) & 0xff) as f32 / 255.0
}
