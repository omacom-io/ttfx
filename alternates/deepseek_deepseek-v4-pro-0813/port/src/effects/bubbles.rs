
use super::Effect;

use crate::engine::canvas::{Canvas, Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::Color;

pub struct Bubbles;

impl Bubbles {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Bubbles {
    fn name(&self) -> &str {
        "bubbles"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        if input.is_empty() {
            return vec![];
        }

        let lines: Vec<&str> = input.lines().collect();
        let width = lines
            .iter()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0)
            .max(1) as u16;
        let height = lines.len().max(1) as u16;

        let terminal = Terminal::from_input(input, width, height);
        let Terminal {
            mut canvas,
            characters,
            ..
        } = terminal;

        if characters.is_empty() {
            return vec![];
        }

        let bubble_size = 4usize;
        let char_count = characters.len();
        let num_bubbles = (char_count + bubble_size - 1) / bubble_size;
        let radius = if width < 10 || height < 10 {
            1.0_f32
        } else {
            2.0_f32
        };

        struct CharData {
            symbol: String,
            input_pos: Coord,
            start_pos: Coord,
            pop_pos: Coord,
        }

        let char_data: Vec<CharData> = characters
            .iter()
            .enumerate()
            .map(|(i, character)| {
                let bubble_index = i / bubble_size;
                let group_start = bubble_index * bubble_size;
                let group_len = (char_count - group_start).min(bubble_size);
                let within = i - group_start;
                let angle = if group_len > 1 {
                    within as f32 / group_len as f32 * 2.0 * std::f32::consts::PI
                } else {
                    0.0
                };
                let offset = Coord::new(angle.cos() * radius, angle.sin() * radius);

                let min_x = radius.min(width as f32 / 2.0);
                let max_x = (width as f32 - radius - 1.0).max(min_x);
                let center_x =
                    (bubble_index as f32 + 0.5) * (width as f32 / num_bubbles as f32);
                let center_x = center_x.clamp(min_x, max_x);

                let start_center = Coord::new(center_x, height as f32 + radius + 2.0);
                let pop_center = Coord::new(center_x, 2.0_f32.max(radius + 0.5));

                CharData {
                    symbol: character.input_symbol.clone(),
                    input_pos: character.position,
                    start_pos: start_center + offset,
                    pop_pos: pop_center + offset,
                }
            })
            .collect();

        fn draw_cell(
            canvas: &mut Canvas,
            symbol: &str,
            pos: Coord,
            style: CellStyle,
            width: u16,
            height: u16,
        ) {
            let x = pos.x.round() as i32;
            let y = pos.y.round() as i32;
            if x >= 0 && y >= 0 {
                let x = x as u16;
                let y = y as u16;
                if x < width && y < height {
                    canvas.set_cell(x, y, Cell::new(symbol, style));
                }
            }
        }

        let mut frames = Vec::new();

        let mut rise_style = CellStyle::new(Color::CYAN, Color::BLACK);
        rise_style.bold = true;
        let pop_style = CellStyle::new(Color::WHITE, Color::BLACK);

        let rise_frames = 45usize;
        let pop_frames = 25usize;

        for frame in 0..rise_frames {
            canvas.clear();
            let progress = if rise_frames > 1 {
                frame as f32 / (rise_frames - 1) as f32
            } else {
                1.0
            };
            let eased = easing::ease_in_out_quad(progress);
            for data in &char_data {
                let pos = data.start_pos.lerp(data.pop_pos, eased);
                draw_cell(&mut canvas, &data.symbol, pos, rise_style, width, height);
            }
            frames.push(canvas.render_frame());
        }

        for frame in 0..=pop_frames {
            canvas.clear();
            let progress = if pop_frames > 0 {
                frame as f32 / pop_frames as f32
            } else {
                1.0
            };
            let eased = easing::ease_out_cubic(progress);
            for data in &char_data {
                let pos = data.pop_pos.lerp(data.input_pos, eased);
                draw_cell(&mut canvas, &data.symbol, pos, pop_style, width, height);
            }
            frames.push(canvas.render_frame());
        }

        frames
    }
}
