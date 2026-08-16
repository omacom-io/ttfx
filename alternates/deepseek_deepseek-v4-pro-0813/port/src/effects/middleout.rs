
use super::Effect;
use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::Coord;

pub struct Middleout;

impl Middleout {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Middleout {
    fn name(&self) -> &str {
        "middleout"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (mut width, mut height) = Terminal::autodetect_size();
        if width == 0 {
            width = 80;
        }
        if height == 0 {
            height = 24;
        }

        let mut terminal = Terminal::from_input(input, width, height);

        if terminal.characters.is_empty() {
            return vec![terminal.write_frame()];
        }

        let center_row = height / 2;

        let char_infos: Vec<(String, Coord, Coord)> = terminal
            .characters
            .iter()
            .map(|c| {
                let center = Coord::new(c.position.x, center_row as f32);
                (c.input_symbol.clone(), c.position, center)
            })
            .collect();

        let center_frames = 12_u32;
        let full_frames = 48_u32;
        let total_frames = center_frames + full_frames;

        let mut frames = Vec::with_capacity(total_frames as usize);

        for frame_idx in 0..total_frames {
            terminal.clear_canvas();

            if frame_idx >= center_frames {
                let local = frame_idx - center_frames;
                let t = if full_frames > 1 {
                    local as f32 / (full_frames - 1) as f32
                } else {
                    1.0
                };
                let eased = easing::ease_in_out_sine(t);

                for (symbol, input_pos, center_pos) in &char_infos {
                    let pos = center_pos.lerp(*input_pos, eased);
                    if let Some((x, y)) = coord_to_cell(pos, width, height) {
                        terminal
                            .canvas
                            .set_cell(x, y, Cell::new(symbol.clone(), CellStyle::default()));
                    }
                }
            }

            frames.push(terminal.write_frame());
        }

        frames
    }
}

fn coord_to_cell(coord: Coord, width: u16, height: u16) -> Option<(u16, u16)> {
    if coord.x < 0.0 || coord.y < 0.0 {
        return None;
    }

    let x = coord.x.round();
    let y = coord.y.round();

    if x >= width as f32 || y >= height as f32 {
        return None;
    }

    Some((x as u16, y as u16))
}
