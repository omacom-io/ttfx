
use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Gradient};

use super::Effect;

fn edge_coord(seed: u32, width: u16, height: u16) -> Coord {
    let w = width.max(1) as f32;
    let h = height.max(1) as f32;
    let perimeter = 2.0 * (w + h);
    let d = ((seed % 10007) as f32 * 12.9898) % perimeter;

    if d < w {
        Coord::new(d, 0.0)
    } else if d < w + h {
        Coord::new(w - 1.0, d - w)
    } else if d < w + h + w {
        Coord::new(w - 1.0 - (d - w - h), h - 1.0)
    } else {
        Coord::new(0.0, h - 1.0 - (d - w - h - w))
    }
}

pub struct Swarm;

impl Swarm {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Swarm {
    fn name(&self) -> &str {
        "swarm"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (width, height) = Terminal::autodetect_size();
        let mut terminal = Terminal::from_input(input, width, height);

        let characters = terminal.characters.clone();
        let n = characters.len().max(1) as f32;

        let input_positions: Vec<Coord> = characters.iter().map(|c| c.position).collect();
        let input_symbols: Vec<String> = characters.iter().map(|c| c.input_symbol.clone()).collect();

        let swarm_symbols: [char; 8] = ['+', '*', 'o', '●', '○', '·', '✱', 'x'];

        let gradient = Gradient::new()
            .add_stop(0.0, Color::new(255, 220, 80))
            .add_stop(0.6, Color::new(255, 160, 40))
            .add_stop(1.0, Color::WHITE);

        let total_frames: usize = 90;
        let mut frames = Vec::with_capacity(total_frames);

        for frame_index in 0..total_frames {
            terminal.clear_canvas();

            let progress = (frame_index as f32) / ((total_frames - 1) as f32).max(1.0);

            for (idx, ch) in characters.iter().enumerate() {
                let target = input_positions[idx];

                let stagger = (idx as f32 / n) * 0.25;
                let local_progress = ((progress - stagger) / (1.0 - stagger)).clamp(0.0, 1.0);
                let eased = easing::ease_in_out_quad(local_progress);

                let is_blank = input_symbols[idx].trim().is_empty();
                if is_blank && eased < 0.95 {
                    continue;
                }

                if eased < 0.85 {
                    let start = edge_coord(ch.id, width, height);
                    let mut pos = start.lerp(target, eased);

                    let dx = target.x - start.x;
                    let dy = target.y - start.y;
                    let len = (dx * dx + dy * dy).sqrt();
                    if len > 0.01 {
                        let wobble = (1.0 - eased) * 8.0;
                        let wave = (ch.id as f32 + frame_index as f32 * 0.4).sin();
                        let nx = -dy / len;
                        let ny = dx / len;
                        pos.x += nx * wave * wobble;
                        pos.y += ny * wave * wobble;
                    }

                    let x = pos.x.round();
                    let y = pos.y.round();
                    if x < 0.0 || y < 0.0 || x >= width as f32 || y >= height as f32 {
                        continue;
                    }

                    let sym_index = (ch.id as usize + frame_index / 2) % swarm_symbols.len();
                    let style = CellStyle::new(gradient.color_at(eased), Color::BLACK);

                    terminal.canvas.set_cell(
                        x as u16,
                        y as u16,
                        Cell::new(swarm_symbols[sym_index].to_string(), style),
                    );
                } else {
                    let x = target.x.round();
                    let y = target.y.round();
                    if x < 0.0 || y < 0.0 || x >= width as f32 || y >= height as f32 {
                        continue;
                    }

                    terminal.canvas.set_cell(
                        x as u16,
                        y as u16,
                        Cell::new(input_symbols[idx].clone(), terminal.config.default_style),
                    );
                }
            }

            frames.push(terminal.write_frame());
        }

        frames
    }
}
