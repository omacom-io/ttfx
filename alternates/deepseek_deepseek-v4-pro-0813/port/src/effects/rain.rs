use super::Effect;
use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::graphics::Color;

const RAIN_COLORS: [Color; 5] = [
    Color::GREEN,
    Color::new(0, 200, 0),
    Color::CYAN,
    Color::new(0, 128, 0),
    Color::BLUE,
];

const RAIN_SYMBOLS: &[&str] = &["│", "┃", "╽", "┆", "┇", "┊", "┋", "|"];

const FRAMES_PER_DROP: f32 = 28.0;
const MAX_DROP_DELAY: f32 = 22.0;
const TRAIL_LENGTH: usize = 10;

pub struct Rain;

impl Rain {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Rain {
    fn name(&self) -> &str {
        "rain"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let width = input
            .lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(1)
            .max(1) as u16;
        let height = input.lines().count().max(1) as u16;

        let mut terminal = Terminal::from_input(input, width, height);
        let mut characters = std::mem::take(&mut terminal.characters);

        let total_frames = (MAX_DROP_DELAY + FRAMES_PER_DROP) as usize + 12;
        let mut frames = Vec::with_capacity(total_frames);

        for frame_index in 0..total_frames {
            terminal.canvas.clear();
            let time = frame_index as f32;

            for character in &mut characters {
                let id = character.id;

                // Deterministic per-character delay in [0, MAX_DROP_DELAY].
                let delay = ((id as f32 * 1.618_033_988_7).fract() * MAX_DROP_DELAY).round();
                let start_y = -((id % 23) as f32 + 6.0);

                let target_x = character.position.x;
                let target_y = character.position.y;

                let progress = (time - delay) / FRAMES_PER_DROP;

                if progress >= 1.0 {
                    let style = CellStyle::default();
                    terminal.canvas.set_cell(
                        target_x as u16,
                        target_y as u16,
                        Cell::new(character.input_symbol.clone(), style),
                    );
                    continue;
                }

                if progress < 0.0 {
                    continue;
                }

                let eased = easing::ease_in_sine(progress.clamp(0.0, 1.0));
                let y = start_y + (target_y - start_y) * eased;
                let head_x = target_x;
                let head_y = y.round() as i32;

                for trail_offset in 0..TRAIL_LENGTH {
                    let trail_y = head_y - trail_offset as i32;

                    if trail_y < 0 || trail_y as u16 >= terminal.config.height {
                        continue;
                    }
                    if head_x < 0.0 || head_x as u16 >= terminal.config.width {
                        continue;
                    }

                    let symbol_index = (id as usize + trail_offset) % RAIN_SYMBOLS.len();
                    let color_index = (id as usize + trail_offset) % RAIN_COLORS.len();

                    let style = CellStyle::new(RAIN_COLORS[color_index], Color::BLACK);
                    terminal.canvas.set_cell(
                        head_x as u16,
                        trail_y as u16,
                        Cell::new(RAIN_SYMBOLS[symbol_index], style),
                    );
                }
            }

            frames.push(terminal.write_frame());
        }

        frames
    }
}
