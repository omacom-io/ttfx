use super::Effect;
use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::easing::{self, EasingFn};
use crate::utils::geometry::Coord;

pub struct Expand {
    movement_speed: f32,
    expand_easing: EasingFn,
}

impl Expand {
    pub fn new() -> Self {
        Self {
            movement_speed: 1.0,
            expand_easing: easing::ease_out_expo,
        }
    }

    fn input_dimensions(input: &str) -> (u16, u16) {
        let mut width = 0u16;
        let mut height = 1u16;
        let mut line_width = 0u16;

        for ch in input.chars() {
            if ch == '\n' {
                height = height.saturating_add(1);
                width = width.max(line_width);
                line_width = 0;
            } else {
                line_width = line_width.saturating_add(1);
            }
        }

        (width.max(line_width).max(1), height)
    }

    fn animate(&self, input: &str) -> Vec<String> {
        let (width, height) = Self::input_dimensions(input);
        let mut terminal = Terminal::from_input(input, width, height);

        let center = Coord::new((width as f32 - 1.0) / 2.0, (height as f32 - 1.0) / 2.0);

        let snapshot: Vec<(String, Coord)> = terminal
            .characters
            .iter()
            .map(|c| (c.input_symbol.clone(), c.position))
            .collect();

        if snapshot.is_empty() {
            return vec![terminal.write_frame()];
        }

        let speed = if self.movement_speed <= 0.0 {
            1.0
        } else {
            self.movement_speed
        };

        let base_frame_count = 30usize;
        let frame_count = ((base_frame_count as f32 / speed).round() as usize).max(1);

        let mut frames = Vec::with_capacity(frame_count + 1);

        for i in 0..=frame_count {
            let t = i as f32 / frame_count as f32;
            let eased_t = (self.expand_easing)(t);

            terminal.clear_canvas();

            for (symbol, dest) in &snapshot {
                let current = center.lerp(*dest, eased_t);
                let x = current.x.round().clamp(0.0, width as f32 - 1.0) as u16;
                let y = current.y.round().clamp(0.0, height as f32 - 1.0) as u16;

                terminal
                    .canvas
                    .set_cell(x, y, Cell::new(symbol.clone(), CellStyle::default()));
            }

            frames.push(terminal.write_frame());
        }

        frames
    }
}

impl Effect for Expand {
    fn name(&self) -> &str {
        "expand"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        self.animate(input)
    }
}
