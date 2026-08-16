use crate::engine::canvas::Cell;
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::Coord;
use super::Effect;

pub struct Scattered {
    frames_count: usize,
}

impl Scattered {
    pub fn new() -> Self {
        Self { frames_count: 30 }
    }
}

impl Effect for Scattered {
    fn name(&self) -> &str {
        "scattered"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (width, height) = Terminal::autodetect_size();
        let width = width.max(1);
        let height = height.max(1);

        let mut terminal = Terminal::from_input(input, width, height);
        let style = terminal.config.default_style;
        let mut seed: u32 = 0x1234_5678;

        let mut characters = Vec::new();
        for character in &terminal.characters {
            let start = if width < 2 || height < 2 {
                Coord::new(1.0, 1.0)
            } else {
                random_coord(&mut seed, width, height)
            };
            characters.push((character.input_symbol.clone(), start, character.position));
        }

        let mut frames = Vec::with_capacity(self.frames_count);
        for frame_index in 0..self.frames_count {
            terminal.clear_canvas();

            let t = if self.frames_count > 1 {
                frame_index as f32 / (self.frames_count - 1) as f32
            } else {
                1.0
            };
            let eased = easing::ease_out_quad(t);

            for (symbol, start, end) in &characters {
                let pos = start.lerp(*end, eased);
                let x = pos.x.round().max(0.0).min((width - 1) as f32) as u16;
                let y = pos.y.round().max(0.0).min((height - 1) as f32) as u16;
                terminal.canvas.set_cell(x, y, Cell::new(symbol.clone(), style));
            }

            frames.push(terminal.write_frame());
        }

        frames
    }
}

fn next_rand(seed: &mut u32) -> u32 {
    *seed ^= (*seed).wrapping_shl(13);
    *seed ^= (*seed).wrapping_shr(17);
    *seed ^= (*seed).wrapping_shl(5);
    *seed
}

fn random_coord(seed: &mut u32, width: u16, height: u16) -> Coord {
    let x = (next_rand(seed) % width as u32) as u16;
    let y = (next_rand(seed) % height as u32) as u16;
    Coord::new(x as f32, y as f32)
}
