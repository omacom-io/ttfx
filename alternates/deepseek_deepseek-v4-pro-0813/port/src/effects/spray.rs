use super::Effect;
use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::Coord;

pub struct Spray;

impl Spray {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Spray {
    fn name(&self) -> &str {
        "spray"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        // Determine canvas size from the input.
        let lines: Vec<&str> = input.lines().collect();
        if lines.is_empty() {
            return Vec::new();
        }

        let width = lines.iter().map(|line| line.chars().count()).max().unwrap_or(0) as u16;
        let height = lines.len() as u16;
        if width == 0 || height == 0 {
            return Vec::new();
        }

        let mut terminal = Terminal::from_input(input, width, height);

        struct Drop {
            symbol: String,
            target: Coord,
            id: u32,
        }

        let drops: Vec<Drop> = terminal.characters.iter().map(|c| Drop {
            symbol: c.output_symbol.clone(),
            target: Coord::new(c.position.x, c.position.y),
            id: c.id,
        }).collect();

        // Start each droplet at a nozzle centred near the bottom of the canvas.
        let start = Coord::new(width as f32 / 2.0, height as f32 - 1.0);

        let total_frames = 45usize;
        let mut frames = Vec::with_capacity(total_frames);

        for frame_idx in 0..total_frames {
            let p = frame_idx as f32 / (total_frames - 1) as f32;
            let eased = easing::ease_out_cubic(p);

            terminal.clear_canvas();

            for drop in &drops {
                let base = start.lerp(drop.target, eased);

                // Deterministic spray spread that converges to zero by the final frame.
                let phase = drop.id as f32 * 12.9898;
                let angle = drop.id as f32 * 2.399;
                let spread = phase.sin() * 2.0 * (1.0 - p).powi(2);
                let pos = Coord::new(
                    base.x + spread * angle.cos(),
                    base.y + spread * angle.sin(),
                );

                let x = pos.x.round().clamp(0.0, (width - 1) as f32) as u16;
                let y = pos.y.round().clamp(0.0, (height - 1) as f32) as u16;

                let style = CellStyle::default();
                terminal.canvas.set_cell(x, y, Cell::new(drop.symbol.clone(), style));
            }

            frames.push(terminal.write_frame());
        }

        frames
    }
}
