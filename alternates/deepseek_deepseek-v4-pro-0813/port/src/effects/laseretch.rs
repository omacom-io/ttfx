use super::Effect;
use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::character::EffectCharacter;
use crate::engine::terminal::Terminal;
use crate::utils::geometry::Coord;
use crate::utils::graphics::Color;

/// Minimal deterministic RNG used for spark placement.
struct SimpleRng {
    state: u32,
}

impl SimpleRng {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(1664525)
            .wrapping_add(1013904223);
        self.state
    }

    fn range(&mut self, low: i32, high: i32) -> i32 {
        if high <= low {
            return low;
        }
        let span = (high - low + 1) as u32;
        low + (self.next_u32() % span) as i32
    }
}

fn coord_to_cell(coord: Coord, width: u16, height: u16) -> Option<(u16, u16)> {
    let x = coord.x.round();
    let y = coord.y.round();
    if x < 0.0 || y < 0.0 {
        return None;
    }
    let x = x as u16;
    let y = y as u16;
    if x >= width || y >= height {
        return None;
    }
    Some((x, y))
}

fn style_for_age(age: u32, default_style: CellStyle, laser_color: Color) -> CellStyle {
    let mut style = default_style;
    style.bold = age == 0;
    style.fg = match age {
        0 => laser_color,
        1 => Color::new(255, 140, 0),
        2 => Color::new(255, 210, 80),
        _ => default_style.fg,
    };
    style
}

pub struct Laseretch;

impl Laseretch {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Laseretch {
    fn name(&self) -> &str {
        "laseretch"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (width, height) = Terminal::autodetect_size();
        let mut terminal = Terminal::from_input(input, width, height);
        let char_count = terminal.characters.len();

        if char_count == 0 {
            return vec![terminal.write_frame()];
        }

        for character in &mut terminal.characters {
            character.set_visibility(false);
        }
        terminal.clear_canvas();

        let chars = terminal.characters.clone();
        let default_style = terminal.config.default_style;
        let laser_color = Color::new(255, 64, 0);
        let mut rng = SimpleRng::new(0xA5A5_5A5A);
        let mut frames = Vec::new();

        // Start with a blank screen.
        frames.push(terminal.write_frame());

        for i in 0..char_count {
            let current_pos = chars[i].position;

            // Three subframes per character create a short laser heat trail.
            for step in 0..3_u32 {
                terminal.clear_canvas();
                let mut occupied: Vec<(u16, u16)> = Vec::new();

                for (j, ch) in chars.iter().enumerate().take(i + 1) {
                    let age = (i - j) as u32 + step;
                    let style = style_for_age(age, default_style, laser_color);

                    if let Some((x, y)) = coord_to_cell(ch.position, width, height) {
                        terminal
                            .canvas
                            .set_cell(x, y, Cell::new(ch.input_symbol.clone(), style));
                        occupied.push((x, y));
                    }
                }

                // Emit a few sparks around the current laser position.
                let spark_symbols = ["*", ".", "+", "`", "'"];
                for _ in 0..2 {
                    let dx = rng.range(-1, 1);
                    let dy = rng.range(-1, 1);
                    if dx == 0 && dy == 0 {
                        continue;
                    }

                    let sx = current_pos.x as i32 + dx;
                    let sy = current_pos.y as i32 + dy;

                    if sx < 0 || sy < 0 || sx >= width as i32 || sy >= height as i32 {
                        continue;
                    }

                    let spark_pos = (sx as u16, sy as u16);
                    if occupied.contains(&spark_pos) {
                        continue;
                    }

                    let idx = rng.range(0, spark_symbols.len() as i32 - 1) as usize;
                    let spark_symbol = spark_symbols[idx].to_string();
                    let spark_style = CellStyle::new(laser_color, default_style.bg);

                    terminal.canvas.set_cell(
                        spark_pos.0,
                        spark_pos.1,
                        Cell::new(spark_symbol, spark_style),
                    );
                    occupied.push(spark_pos);
                }

                frames.push(terminal.write_frame());
            }
        }

        // Let the laser trail cool after all characters have been etched.
        let last_idx = char_count - 1;
        for cool_step in 0..4_u32 {
            terminal.clear_canvas();

            for (j, ch) in chars.iter().enumerate() {
                let age = (last_idx - j) as u32 + 3 + cool_step;
                let style = style_for_age(age, default_style, laser_color);

                if let Some((x, y)) = coord_to_cell(ch.position, width, height) {
                    terminal
                        .canvas
                        .set_cell(x, y, Cell::new(ch.input_symbol.clone(), style));
                }
            }

            frames.push(terminal.write_frame());
        }

        // Final frame with entirely normal terminal styling.
        terminal.clear_canvas();
        for ch in &chars {
            if let Some((x, y)) = coord_to_cell(ch.position, width, height) {
                terminal
                    .canvas
                    .set_cell(x, y, Cell::new(ch.input_symbol.clone(), default_style));
            }
        }
        frames.push(terminal.write_frame());

        frames
    }
}
