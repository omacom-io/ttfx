use super::Effect;
use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::character::EffectCharacter;
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::Color;

pub struct Smoke;

impl Smoke {
    pub fn new() -> Self {
        Self
    }
}

struct Lcg(u32);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        self.0
    }

    fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (self.next() as f32 / u32::MAX as f32) * (high - low)
    }
}

struct Particle {
    input_symbol: String,
    start: Coord,
    target: Coord,
    start_frame: usize,
    duration: usize,
    done: bool,
}

impl Effect for Smoke {
    fn name(&self) -> &str {
        "smoke"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        if input.is_empty() {
            let terminal = Terminal::new(10, 10);
            return vec![terminal.write_frame()];
        }

        let lines: Vec<&str> = input.lines().collect();
        let line_count = lines.len().max(1) as u16;
        let max_line_len = lines
            .iter()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0)
            .max(1) as u16;

        let margin_x: u16 = 4;
        let top_margin: u16 = 2;
        let bottom_margin: u16 = 1;
        let extra_height: u16 = 6;

        let width = max_line_len + margin_x * 2 + 4;
        let height = line_count + top_margin + bottom_margin + extra_height;

        let mut terminal = Terminal::new(width, height);
        let mut particles = Vec::new();
        let mut lcg = Lcg(0x72616E64); // arbitrary deterministic seed
        let mut next_id: u32 = 0;

        // Anchor text near the bottom so the smoke has room to rise.
        let anchor_y = height - bottom_margin - line_count;

        for (line_index, line) in lines.iter().enumerate() {
            for (col_index, ch) in line.chars().enumerate() {
                let x = margin_x + col_index as u16;
                let y = anchor_y + line_index as u16;
                let coord = Coord::new(x as f32, y as f32);

                let character = EffectCharacter::new(next_id, ch.to_string(), coord);
                terminal.characters.push(character);
                terminal
                    .canvas
                    .set_cell(x, y, Cell::new(ch.to_string(), terminal.config.default_style));
                next_id += 1;

                particles.push(Particle {
                    input_symbol: ch.to_string(),
                    start: coord,
                    target: Coord::new(coord.x + lcg.range(-6.0, 6.0), top_margin as f32 - 2.0),
                    start_frame: lcg.next() as usize % 18,
                    duration: 14 + (lcg.next() as usize % 16),
                    done: false,
                });
            }
        }

        let smoke_symbols = ["█", "▓", "▒", "░", " "];
        let mut frames = Vec::new();
        let mut frame_index = 0usize;
        let max_frames = 160;

        loop {
            terminal.clear_canvas();
            let mut all_done = true;

            for particle in &mut particles {
                if particle.done {
                    continue;
                }

                if frame_index < particle.start_frame {
                    all_done = false;
                    let x = particle.start.x.round() as u16;
                    let y = particle.start.y.round() as u16;
                    if x < width && y < height {
                        terminal.canvas.set_cell(
                            x,
                            y,
                            Cell::new(particle.input_symbol.clone(), terminal.config.default_style),
                        );
                    }
                    continue;
                }

                let elapsed = (frame_index - particle.start_frame) as f32;
                let progress = elapsed / particle.duration as f32;
                if progress >= 1.0 {
                    particle.done = true;
                    continue;
                }

                all_done = false;

                let eased = easing::ease_out_quart(progress);
                let x = particle.start.x + (particle.target.x - particle.start.x) * eased;
                let y = particle.start.y + (particle.target.y - particle.start.y) * eased;
                let symbol_index = ((eased * (smoke_symbols.len() - 1) as f32).round() as usize)
                    .min(smoke_symbols.len() - 1);
                let v = 220.0 - eased * 180.0;
                let fg = Color::new(v as u8, v as u8, v as u8);
                let style = CellStyle::new(fg, Color::BLACK);

                if x >= 0.0 && y >= 0.0 {
                    let cx = x.round() as u16;
                    let cy = y.round() as u16;
                    if cx < width && cy < height {
                        terminal
                            .canvas
                            .set_cell(cx, cy, Cell::new(smoke_symbols[symbol_index], style));
                    }
                }
            }

            frames.push(terminal.write_frame());
            frame_index += 1;

            if all_done || frame_index >= max_frames {
                break;
            }
        }

        frames
    }
}
