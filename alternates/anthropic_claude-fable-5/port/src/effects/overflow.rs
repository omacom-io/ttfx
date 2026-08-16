//! Overflow: the input text overflows the terminal and scrolls past in a random
//! row order for a few cycles until the rows eventually arrive in the correct
//! order and settle into place, revealing the final gradient-colored text.
//!
//! Port of `terminaltexteffects/effects/effect_overflow.py`:
//! - Rows of the input are copied `overflow_cycles_range` times in shuffled
//!   order and queued ahead of the true (final) rows.
//! - Each frame (subject to a random delay) pops 1..=overflow_speed rows from
//!   the queue; every pop scrolls all active rows up one line and enters the
//!   new row at the bottom, exactly like terminal output scrolling.
//! - Scrolling rows are tinted from the pale overflow gradient by their
//!   current row; final rows carry their final gradient colors, so the text
//!   visibly "resolves" as the last cycle scrolls in and stops.

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::character::EffectCharacter;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Minimal xorshift PRNG with Python-shaped helpers (randint is inclusive,
/// shuffle is Fisher-Yates like `random.shuffle`).
struct Rng {
    state: u64,
}

impl Rng {
    fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Rng { state: seed | 1 }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Inclusive range, like Python's `random.randint(low, high)`.
    fn randint(&mut self, low: i64, high: i64) -> i64 {
        if high <= low {
            return low;
        }
        let span = (high - low + 1) as u64;
        low + (self.next_u64() % span) as i64
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        if items.len() < 2 {
            return;
        }
        for i in (1..items.len()).rev() {
            let j = self.randint(0, i as i64) as usize;
            items.swap(i, j);
        }
    }
}

/// One horizontal row of characters, referenced by arena index.
/// Mirrors `OverflowIterator.Row` from the Python effect.
struct Row {
    character_indices: Vec<usize>,
    is_final: bool,
}

/// Color a row: final rows show their final gradient colors; overflow copies
/// are tinted from the overflow gradient by their current canvas row.
fn apply_row_color(
    terminal: &mut Terminal,
    row: &Row,
    overflow_spectrum: &[Color],
    final_colors: &[Option<Color>],
) {
    for &idx in &row.character_indices {
        let color = if row.is_final {
            final_colors.get(idx).copied().flatten()
        } else {
            let current_row = terminal.characters[idx].motion.current_coord.row;
            let clamped = ((current_row - 1).max(0) as usize)
                .min(overflow_spectrum.len().saturating_sub(1));
            overflow_spectrum.get(clamped).copied()
        };
        let character = &mut terminal.characters[idx];
        character.animation.current_visual =
            CharacterVisual::new(character.input_symbol, false, ColorPair::new(color, None));
    }
}

/// Scroll every active row up one line (Python `Row.move_up`), retint the
/// overflow rows for their new position and drop copies that left the canvas.
fn scroll_rows_up(
    terminal: &mut Terminal,
    active_rows: &mut Vec<Row>,
    overflow_spectrum: &[Color],
    final_colors: &[Option<Color>],
    canvas_top: i32,
) {
    for row in active_rows.iter() {
        for &idx in &row.character_indices {
            let coord = terminal.characters[idx].motion.current_coord;
            terminal.characters[idx].motion.current_coord =
                Coord::new(coord.column, coord.row + 1);
        }
    }
    for row in active_rows.iter() {
        apply_row_color(terminal, row, overflow_spectrum, final_colors);
    }
    // Overflow copies that scrolled past the top are done; the canvas clips
    // them anyway, this just keeps the active set small. Final rows are kept.
    active_rows.retain(|row| {
        if row.is_final {
            return true;
        }
        !row
            .character_indices
            .iter()
            .all(|&idx| terminal.characters[idx].motion.current_coord.row > canvas_top)
    });
}

pub struct Overflow {
    final_gradient_stops: Vec<Color>,
    final_gradient_steps: usize,
    overflow_gradient_stops: Vec<Color>,
    overflow_cycles_range: (u32, u32),
    overflow_speed: u32,
}

impl Overflow {
    pub fn new() -> Self {
        Overflow {
            // Python defaults: 8A008A, 00D1FF, FFFFFF
            final_gradient_stops: vec![
                Color::new(0x8A, 0x00, 0x8A),
                Color::new(0x00, 0xD1, 0xFF),
                Color::new(0xFF, 0xFF, 0xFF),
            ],
            final_gradient_steps: 12,
            // Python defaults: f2ebc0, 8dbfb3, f2ebc0
            overflow_gradient_stops: vec![
                Color::new(0xF2, 0xEB, 0xC0),
                Color::new(0x8D, 0xBF, 0xB3),
                Color::new(0xF2, 0xEB, 0xC0),
            ],
            overflow_cycles_range: (2, 4),
            overflow_speed: 3,
        }
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
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let canvas_top = terminal.canvas.height as i32;
        let mut rng = Rng::new();

        // --- build(): final gradient mapping (vertical, bottom -> top). ---
        let final_gradient = Gradient::new(&self.final_gradient_stops, self.final_gradient_steps);
        let final_colors: Vec<Option<Color>> = terminal
            .get_characters()
            .iter()
            .map(|c| {
                let fraction = if canvas_top > 1 {
                    (c.input_coord.row - 1) as f64 / (canvas_top - 1) as f64
                } else {
                    0.0
                };
                final_gradient.get_color_at_fraction(fraction)
            })
            .collect();

        // Group the original characters into rows, top to bottom. Empty rows
        // are kept as empty groups so vertical gaps in the input survive the
        // scroll (each row, blank or not, consumes exactly one scroll slot).
        let mut base_rows: Vec<Vec<usize>> = Vec::new();
        for row in (1..=canvas_top).rev() {
            let indices: Vec<usize> = terminal
                .get_characters()
                .iter()
                .enumerate()
                .filter(|(_, c)| c.input_coord.row == row)
                .map(|(i, _)| i)
                .collect();
            base_rows.push(indices);
        }

        // Queue shuffled copies of every row for each overflow cycle.
        let mut pending_rows: VecDeque<Row> = VecDeque::new();
        let cycles = rng.randint(
            self.overflow_cycles_range.0 as i64,
            self.overflow_cycles_range.1 as i64,
        );
        let mut shuffled_rows = base_rows.clone();
        for _ in 0..cycles {
            rng.shuffle(&mut shuffled_rows);
            for row in &shuffled_rows {
                let mut copied_indices = Vec::with_capacity(row.len());
                for &idx in row {
                    let (symbol, coord) = {
                        let original = &terminal.characters[idx];
                        (original.input_symbol, original.input_coord)
                    };
                    let new_id = terminal.characters.len();
                    terminal
                        .characters
                        .push(EffectCharacter::new(new_id, symbol, coord));
                    copied_indices.push(new_id);
                }
                pending_rows.push_back(Row {
                    character_indices: copied_indices,
                    is_final: false,
                });
            }
        }
        // The true rows, in correct order, go last.
        for row in &base_rows {
            pending_rows.push_back(Row {
                character_indices: row.clone(),
                is_final: true,
            });
        }

        // Overflow tint gradient sized to the canvas height, as in Python:
        // steps = max(canvas.top // max(1, len(stops) - 1), 1)
        let steps = ((canvas_top / (self.overflow_gradient_stops.len() as i32 - 1).max(1))
            .max(1)) as usize;
        let overflow_gradient = Gradient::new(&self.overflow_gradient_stops, steps);
        let overflow_spectrum = overflow_gradient.spectrum.clone();

        // --- __next__ loop ---
        let mut active_rows: Vec<Row> = Vec::new();
        let mut frames_out: Vec<String> = Vec::new();
        let mut delay: i64 = 0;

        while !pending_rows.is_empty() {
            if delay == 0 {
                let pops = rng.randint(1, self.overflow_speed.max(1) as i64);
                for _ in 0..pops {
                    if let Some(next_row) = pending_rows.pop_front() {
                        // Every entering row scrolls the screen up one line.
                        scroll_rows_up(
                            &mut terminal,
                            &mut active_rows,
                            &overflow_spectrum,
                            &final_colors,
                            canvas_top,
                        );
                        // Enter the new row at the bottom of the canvas.
                        for &idx in &next_row.character_indices {
                            let column = terminal.characters[idx].input_coord.column;
                            terminal.characters[idx].motion.current_coord =
                                Coord::new(column, 1);
                            terminal.characters[idx].is_visible = true;
                        }
                        apply_row_color(
                            &mut terminal,
                            &next_row,
                            &overflow_spectrum,
                            &final_colors,
                        );
                        active_rows.push(next_row);
                    }
                }
                delay = rng.randint(0, 3);
            } else {
                delay -= 1;
            }
            frames_out.push(terminal.get_formatted_output_string());
        }

        // When the queue drains, every final row has scrolled exactly into its
        // input position. Hold the settled text for a moment.
        let settled = terminal.get_formatted_output_string();
        for _ in 0..30 {
            frames_out.push(settled.clone());
        }

        frames_out
    }
}
