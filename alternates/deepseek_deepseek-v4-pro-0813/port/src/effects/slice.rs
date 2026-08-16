use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::easing::{self, EasingFn};
use crate::utils::geometry::Coord;
use super::Effect;

pub struct Slice {
    slice_direction: String,
    movement_speed: f32,
    movement_easing: EasingFn,
}

impl Slice {
    pub fn new() -> Self {
        Self {
            slice_direction: "horizontal".to_string(),
            movement_speed: 0.2,
            movement_easing: easing::ease_out_quad,
        }
    }

    fn terminal_size(input: &str) -> (u16, u16) {
        let lines: Vec<&str> = input.split('\n').collect();
        let height = lines.len() as u16;
        let width = lines
            .iter()
            .map(|line| line.chars().count() as u16)
            .max()
            .unwrap_or(1)
            .max(1);
        (width, height)
    }

    fn start_position(&self, target: Coord, width: u16, height: u16) -> Coord {
        match self.slice_direction.as_str() {
            "horizontal" => {
                if target.x <= width as f32 / 2.0 {
                    Coord::new(target.x, -1.0)
                } else {
                    Coord::new(target.x, height as f32)
                }
            }
            "vertical" => {
                if target.y <= height as f32 / 2.0 {
                    Coord::new(-1.0, target.y)
                } else {
                    Coord::new(width as f32, target.y)
                }
            }
            "diagonal" => {
                if target.x + target.y <= (width as f32 + height as f32) / 2.0 {
                    Coord::new(-1.0, -1.0)
                } else {
                    Coord::new(width as f32, height as f32)
                }
            }
            _ => Coord::new(-1.0, target.y),
        }
    }
}

impl Effect for Slice {
    fn name(&self) -> &str {
        "slice"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        if input.is_empty() {
            return Vec::new();
        }

        let (width, height) = Self::terminal_size(input);
        let mut terminal = Terminal::from_input(input, width, height);

        if terminal.characters.is_empty() {
            return vec![terminal.write_frame()];
        }

        let targets: Vec<Coord> = terminal.characters.iter().map(|c| c.position).collect();
        let starts: Vec<Coord> = targets
            .iter()
            .map(|&target| self.start_position(target, width, height))
            .collect();

        for (i, &start) in starts.iter().enumerate() {
            terminal.characters[i].position = start;
        }

        let max_distance = targets
            .iter()
            .zip(starts.iter())
            .map(|(t, s)| t.distance(*s))
            .fold(0.0f32, |acc, d| acc.max(d));

        let speed = self.movement_speed.clamp(0.05, 1.0);
        let frame_count = ((max_distance / speed).ceil() as usize).max(1).min(240);
        let mut frames = Vec::with_capacity(frame_count + 1);

        for frame_index in 0..=frame_count {
            let t = frame_index as f32 / frame_count as f32;
            let eased_t = (self.movement_easing)(t);

            for i in 0..targets.len() {
                terminal.characters[i].position = starts[i].lerp(targets[i], eased_t);
            }

            let mut placements: Vec<(u16, u16, String, CellStyle)> = Vec::new();
            for character in &terminal.characters {
                if !character.visible {
                    continue;
                }

                let x = character.position.x.round();
                let y = character.position.y.round();

                if x < 0.0 || y < 0.0 || x >= width as f32 || y >= height as f32 {
                    continue;
                }

                placements.push((
                    x as u16,
                    y as u16,
                    character.input_symbol.clone(),
                    character.style,
                ));
            }

            terminal.canvas.clear();
            for (x, y, symbol, style) in placements {
                terminal.canvas.set_cell(x, y, Cell::new(symbol, style));
            }

            frames.push(terminal.write_frame());
        }

        frames
    }
}
