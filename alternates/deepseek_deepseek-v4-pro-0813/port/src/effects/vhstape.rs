
use super::Effect;
use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::graphics::Color;

const NOISE_SYMBOLS: [&str; 8] = ["█", "▓", "▒", "░", "*", ".", "-", " "];
const VHS_COLORS: [Color; 8] = [
    Color::WHITE,
    Color::CYAN,
    Color::MAGENTA,
    Color::YELLOW,
    Color::GREEN,
    Color::RED,
    Color::BLUE,
    Color::WHITE,
];

struct Rng {
    state: u64,
}

impl Rng {
    fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5DEECE66D);

        Self { state: seed | 1 }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn range(&mut self, low: i32, high: i32) -> i32 {
        if high <= low {
            return low;
        }

        let span = (high - low) as u32;
        low + (self.next_u64() % span as u64) as i32
    }
}

fn noise_symbol(rng: &mut Rng) -> &'static str {
    NOISE_SYMBOLS[(rng.next_u64() % NOISE_SYMBOLS.len() as u64) as usize]
}

fn vhs_color(rng: &mut Rng) -> Color {
    VHS_COLORS[(rng.next_u64() % VHS_COLORS.len() as u64) as usize]
}

fn clamp_x(value: i32, max: u16) -> u16 {
    if value < 0 {
        0
    } else if value >= max as i32 {
        max.saturating_sub(1)
    } else {
        value as u16
    }
}

pub struct Vhstape;

impl Vhstape {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Vhstape {
    fn name(&self) -> &str {
        "vhstape"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let width: u16 = 80;
        let height: u16 = 24;
        let mut terminal = Terminal::from_input(input, width, height);

        let base_chars: Vec<(u16, u16, String)> = terminal
            .characters
            .iter()
            .map(|c| {
                (
                    c.position.x as u16,
                    c.position.y as u16,
                    c.input_symbol.clone(),
                )
            })
            .collect();

        let mut rng = Rng::new();
        let mut frames = Vec::new();
        let total_frames = 40;

        for frame_idx in 0..total_frames {
            terminal.clear_canvas();

            // Horizontal VHS glitch bands.
            let mut glitch_shifts: Vec<(u16, i32)> = Vec::new();
            let glitch_bands = 2 + rng.range(0, 4) as usize;

            for _ in 0..glitch_bands {
                let row = rng.range(0, height as i32) as u16;
                let shift = rng.range(-3, 4);
                glitch_shifts.push((row, shift));
            }

            let tracking_row = (frame_idx % height as usize) as u16;
            let tracking_shift = rng.range(-2, 3);

            // Draw the source text, applying glitch displacement and color noise.
            for (x, y, symbol) in &base_chars {
                let mut draw_x = *x as i32;

                for (glitch_row, shift) in &glitch_shifts {
                    if *glitch_row == *y {
                        draw_x += shift;
                    }
                }

                if *y == tracking_row {
                    draw_x += tracking_shift;
                }

                let draw_x = clamp_x(draw_x, width);
                let draw_y = *y;

                let draw_symbol = if rng.next_f32() < 0.08 {
                    noise_symbol(&mut rng).to_string()
                } else {
                    symbol.clone()
                };

                let mut style = CellStyle::new(vhs_color(&mut rng), Color::BLACK);

                if draw_y == tracking_row {
                    style = CellStyle::new(Color::WHITE, Color::BLACK);
                    style.bold = true;
                } else if rng.next_f32() < 0.22 {
                    style.fg = vhs_color(&mut rng);
                }

                terminal
                    .canvas
                    .set_cell(draw_x, draw_y, Cell::new(draw_symbol, style));
            }

            // Add sparse static specks over the frame.
            let speck_count = (width as u32 * height as u32) / 10;
            for _ in 0..speck_count {
                let x = rng.range(0, width as i32) as u16;
                let y = rng.range(0, height as i32) as u16;

                let symbol = noise_symbol(&mut rng);
                let mut style = CellStyle::new(vhs_color(&mut rng), Color::BLACK);
                style.dim = true;

                terminal
                    .canvas
                    .set_cell(x, y, Cell::new(symbol, style));
            }

            // Draw the bright tracking band.
            for x in 0..width {
                let symbol = if (x + tracking_row + frame_idx as u16) % 2 == 0 {
                    " "
                } else {
                    "█"
                };

                let mut style = CellStyle::new(Color::WHITE, Color::BLACK);
                style.reverse = true;

                terminal
                    .canvas
                    .set_cell(x, tracking_row, Cell::new(symbol, style));
            }

            let rendered = terminal.write_frame();
            frames.push(format!("\x1b[2J\x1b[H{rendered}"));
        }

        frames
    }
}
