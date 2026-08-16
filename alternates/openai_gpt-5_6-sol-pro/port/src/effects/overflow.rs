
use super::Effect;
use crate::engine::Terminal;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Style};

const OVERFLOW_CYCLES: usize = 2;
const OVERFLOW_SPEED: usize = 3;
const GRADIENT_STOPS: [(u8, u8, u8); 3] = [
    (0xf2, 0xeb, 0xc0),
    (0x8d, 0xbf, 0xb3),
    (0xf2, 0xeb, 0xc0),
];

#[derive(Debug, Clone)]
pub struct Overflow;

impl Overflow {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Overflow {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Overflow {
    fn name(&self) -> &str {
        "overflow"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let rows = input_rows(input);
        let width = rows
            .iter()
            .map(|row| row.len())
            .max()
            .unwrap_or(0)
            .max(1);
        let height = rows.len().max(1);

        let mut pending_rows = Vec::with_capacity(height * (OVERFLOW_CYCLES + 1));
        let mut random_state = seed_from_input(input);

        for _ in 0..OVERFLOW_CYCLES {
            let mut shuffled = rows.clone();
            shuffle(&mut shuffled, &mut random_state);
            pending_rows.extend(shuffled);
        }

        // The final pass is not shuffled, allowing the original text to settle
        // into its proper row order.
        pending_rows.extend(rows);

        let mut active_rows: Vec<ActiveRow> = Vec::with_capacity(height);
        let mut frames = Vec::new();

        for row in pending_rows {
            for active_row in &mut active_rows {
                active_row.y -= 1;
            }
            active_rows.retain(|active_row| active_row.y >= 0);

            active_rows.push(ActiveRow {
                symbols: row,
                y: height as i32 - 1,
            });

            let frame = render_rows(width, height, &active_rows);

            // Overflow speed controls the time between row advances.
            for _ in 0..OVERFLOW_SPEED {
                frames.push(frame.clone());
            }
        }

        if frames.is_empty() {
            frames.push(Terminal::new(width, height).render_frame());
        }

        frames
    }
}

#[derive(Debug, Clone)]
struct ActiveRow {
    symbols: Vec<char>,
    y: i32,
}

fn input_rows(input: &str) -> Vec<Vec<char>> {
    if input.is_empty() {
        vec![Vec::new()]
    } else {
        input.lines().map(|line| line.chars().collect()).collect()
    }
}

fn render_rows(width: usize, height: usize, rows: &[ActiveRow]) -> String {
    let mut terminal = Terminal::new(width, height);

    for row in rows {
        if row.y < 0 || row.y >= height as i32 {
            continue;
        }

        let color = gradient_color(row.y as usize, height);
        let style = Style::default().with_foreground(color);

        for (x, symbol) in row.symbols.iter().copied().enumerate() {
            if x >= width {
                break;
            }

            let id = terminal.add_character(symbol, Coord::new(x as i32, row.y));
            if let Some(character) = terminal.character_mut(id) {
                character.set_style(style.clone());
            }
        }
    }

    terminal.render_frame()
}

fn gradient_color(row: usize, height: usize) -> Color {
    if height <= 1 {
        let (r, g, b) = GRADIENT_STOPS[0];
        return Color::rgb(r, g, b);
    }

    let progress = row as f64 / (height - 1) as f64;
    let scaled = progress * (GRADIENT_STOPS.len() - 1) as f64;
    let start_index = (scaled.floor() as usize).min(GRADIENT_STOPS.len() - 2);
    let local_progress = scaled - start_index as f64;

    let start = GRADIENT_STOPS[start_index];
    let end = GRADIENT_STOPS[start_index + 1];

    Color::rgb(
        interpolate(start.0, end.0, local_progress),
        interpolate(start.1, end.1, local_progress),
        interpolate(start.2, end.2, local_progress),
    )
}

fn interpolate(start: u8, end: u8, progress: f64) -> u8 {
    let value = start as f64 + (end as f64 - start as f64) * progress;
    value.round().clamp(0.0, 255.0) as u8
}

fn seed_from_input(input: &str) -> u64 {
    let mut seed = 0xcbf2_9ce4_8422_2325_u64;

    for byte in input.bytes() {
        seed ^= u64::from(byte);
        seed = seed.wrapping_mul(0x0000_0100_0000_01b3);
    }

    if seed == 0 {
        0x9e37_79b9_7f4a_7c15
    } else {
        seed
    }
}

fn next_random(state: &mut u64) -> u64 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *state = value;
    value
}

fn shuffle<T>(values: &mut [T], state: &mut u64) {
    for upper in (1..values.len()).rev() {
        let index = (next_random(state) % (upper as u64 + 1)) as usize;
        values.swap(upper, index);
    }
}
