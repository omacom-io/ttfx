use super::Effect;
use crate::engine::canvas::{Canvas, Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::easing::ease_in_out_cubic;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

pub struct Rings;

impl Rings {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Rings {
    fn name(&self) -> &str {
        "rings"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (width, height) = Terminal::autodetect_size();
        let mut terminal = Terminal::from_input(input, width, height);
        let characters = terminal.characters.clone();

        if characters.is_empty() {
            return vec![terminal.write_frame()];
        }

        let gradient = Gradient::new()
            .add_stop(0.0, Color::CYAN)
            .add_stop(0.25, Color::BLUE)
            .add_stop(0.5, Color::MAGENTA)
            .add_stop(0.75, Color::YELLOW)
            .add_stop(1.0, Color::WHITE);

        let positions = ring_positions(characters.len(), width, height, 7.0);

        let max_delay = positions
            .iter()
            .map(|(_, group)| *group as usize * 4)
            .max()
            .unwrap_or(0);

        const HOLD_FRAMES: usize = 15;
        const MOVE_FRAMES: usize = 50;
        const END_FRAMES: usize = 10;

        let total_frames = HOLD_FRAMES + MOVE_FRAMES + max_delay + END_FRAMES;
        let mut frames = Vec::with_capacity(total_frames);

        for frame_idx in 0..=total_frames {
            terminal.clear_canvas();

            for (char_idx, character) in characters.iter().enumerate() {
                let (start, group) = positions[char_idx];
                let delay = group as usize * 4;

                let t = if frame_idx < HOLD_FRAMES + delay {
                    0.0
                } else if frame_idx >= HOLD_FRAMES + delay + MOVE_FRAMES {
                    1.0
                } else {
                    let local = (frame_idx - HOLD_FRAMES - delay) as f32 / MOVE_FRAMES as f32;
                    ease_in_out_cubic(local)
                };

                let current = start.lerp(character.position, t);

                let denom = (width as f32 + height as f32).max(1.0);
                let color = gradient
                    .color_at((character.position.x + character.position.y) / denom);

                let style = CellStyle::with_color_pair(ColorPair::new(color, Color::BLACK));

                draw_coord(
                    &mut terminal.canvas,
                    current,
                    &character.input_symbol,
                    style,
                );
            }

            frames.push(terminal.write_frame());
        }

        frames
    }
}

fn ring_positions(count: usize, width: u16, height: u16, gap: f32) -> Vec<(Coord, usize)> {
    let center = Coord::new(
        (width.saturating_sub(1)) as f32 / 2.0,
        (height.saturating_sub(1)) as f32 / 2.0,
    );

    let mut positions = Vec::with_capacity(count);
    let mut radius = 1.0f32;
    let mut group = 0usize;

    while positions.len() < count {
        let circumference = 2.0 * std::f32::consts::PI * radius;
        let steps = (circumference / 1.2).ceil().max(1.0) as usize;

        for step in 0..steps {
            if positions.len() >= count {
                break;
            }

            let theta = 2.0 * std::f32::consts::PI * step as f32 / steps as f32;
            let x = center.x + radius * theta.cos();
            let y = center.y + radius * theta.sin();

            let max_x = (width.saturating_sub(1)) as f32;
            let max_y = (height.saturating_sub(1)) as f32;

            let clamped_x = x.clamp(0.0, max_x);
            let clamped_y = y.clamp(0.0, max_y);

            positions.push((Coord::new(clamped_x, clamped_y), group));
        }

        radius += gap;
        group += 1;
    }

    positions.truncate(count);
    positions
}

fn draw_coord(canvas: &mut Canvas, coord: Coord, symbol: &str, style: CellStyle) {
    let max_x = (canvas.width.saturating_sub(1)) as f32;
    let max_y = (canvas.height.saturating_sub(1)) as f32;

    let x = coord.x.round().clamp(0.0, max_x) as u16;
    let y = coord.y.round().clamp(0.0, max_y) as u16;

    canvas.set_cell(x, y, Cell::new(symbol, style));
}
