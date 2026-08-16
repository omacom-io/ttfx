use crate::engine::canvas::{Canvas, Cell, CellStyle};
use crate::utils::graphics::{Color, Gradient};

use super::Effect;

#[derive(Clone, Copy)]
enum Direction {
    Vertical,
    Horizontal,
}

pub struct Synthgrid {
    grid_symbol: String,
    grid_color: Color,
    direction: Direction,
    name: &'static str,
}

impl Synthgrid {
    pub fn new() -> Self {
        Self {
            grid_symbol: "▚".to_string(),
            grid_color: Color::GREEN,
            direction: Direction::Vertical,
            name: "synthgrid",
        }
    }
}

impl Effect for Synthgrid {
    fn name(&self) -> &str {
        self.name
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let _ = input;

        let width: u16 = 80;
        let height: u16 = 24;
        let spacing: u16 = 6;
        let total_frames: u16 = 60;

        let mut canvas = Canvas::new(width, height);
        let gradient = Gradient::new()
            .add_stop(0.0, Color::BLACK)
            .add_stop(0.5, self.grid_color)
            .add_stop(1.0, Color::BLACK);

        let mut frames = Vec::with_capacity(total_frames as usize);

        for frame in 0..total_frames {
            canvas.clear();
            let offset = frame % spacing;

            match self.direction {
                Direction::Vertical => {
                    let mut x = offset;
                    while x < width {
                        draw_vertical_line(&mut canvas, x, height, &self.grid_symbol, &gradient);
                        x += spacing;
                    }
                }
                Direction::Horizontal => {
                    let mut y = offset;
                    while y < height {
                        draw_horizontal_line(&mut canvas, y, width, &self.grid_symbol, &gradient);
                        y += spacing;
                    }
                }
            }

            frames.push(canvas.render_frame());
        }

        frames
    }
}

fn draw_vertical_line(
    canvas: &mut Canvas,
    x: u16,
    height: u16,
    symbol: &str,
    gradient: &Gradient,
) {
    if x >= canvas.width {
        return;
    }

    for y in 0..height {
        if y >= canvas.height {
            break;
        }

        let t = if height <= 1 {
            0.5
        } else {
            y as f32 / (height - 1) as f32
        };

        let fg = gradient.color_at(t);
        let style = CellStyle::new(fg, Color::BLACK);
        canvas.set_cell(x, y, Cell::new(symbol, style));
    }
}

fn draw_horizontal_line(
    canvas: &mut Canvas,
    y: u16,
    width: u16,
    symbol: &str,
    gradient: &Gradient,
) {
    if y >= canvas.height {
        return;
    }

    for x in 0..width {
        if x >= canvas.width {
            break;
        }

        let t = if width <= 1 {
            0.5
        } else {
            x as f32 / (width - 1) as f32
        };

        let fg = gradient.color_at(t);
        let style = CellStyle::new(fg, Color::BLACK);
        canvas.set_cell(x, y, Cell::new(symbol, style));
    }
}
