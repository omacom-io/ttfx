//! Synthgrid: a neon grid expands over the canvas, text blocks materialize
//! inside the grid cells with a dissolve animation, then the grid collapses.
//!
//! Port of terminaltexteffects/effects/effect_synthgrid.py.

use std::collections::BTreeMap;

use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::character::EffectCharacter;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

const GRID_ROW_SYMBOL: char = '─';
const GRID_COLUMN_SYMBOL: char = '│';
const TEXT_GENERATION_SYMBOLS: [char; 3] = ['░', '▒', '▓'];
const MAX_ACTIVE_BLOCKS: f64 = 0.1;
const GRADIENT_STEPS: usize = 12;
const FRAME_BUDGET: usize = 20000;

/// Minimal deterministic PRNG (xorshift64) standing in for Python's `random`.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Inclusive range [lo, hi].
    fn gen_range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() as usize) % (hi - lo + 1)
    }

    fn choice<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        &slice[self.gen_range(0, slice.len() - 1)]
    }

    fn shuffle<T>(&mut self, v: &mut [T]) {
        if v.len() < 2 {
            return;
        }
        for i in (1..v.len()).rev() {
            let j = self.gen_range(0, i);
            v.swap(i, j);
        }
    }
}

/// One grid line (horizontal or vertical) made of extra characters pushed
/// into the terminal's character arena. Extended/collapsed one chunk per tick.
struct GridLine {
    /// Indices into `terminal.characters`, in extension order (from origin).
    ids: Vec<usize>,
    /// Characters revealed per tick while extending / hidden per tick while collapsing.
    speed: usize,
    shown: usize,
    hidden: usize,
}

impl GridLine {
    fn extend(&mut self, chars: &mut [EffectCharacter]) {
        for _ in 0..self.speed {
            if self.shown < self.ids.len() {
                chars[self.ids[self.shown]].is_visible = true;
                self.shown += 1;
            }
        }
    }

    fn is_extended(&self) -> bool {
        self.shown >= self.ids.len()
    }

    fn collapse(&mut self, chars: &mut [EffectCharacter]) {
        for _ in 0..self.speed {
            if self.hidden < self.shown {
                chars[self.ids[self.hidden]].is_visible = false;
                self.hidden += 1;
            }
        }
    }

    fn is_collapsed(&self) -> bool {
        self.hidden >= self.shown
    }
}

/// Find a gap size that divides the dimension nearly evenly, closest to
/// one fifth of the dimension. Mirrors SynthGridIterator.find_even_gap.
fn find_even_gap(mut dimension: i32) -> i32 {
    let mut potential_gaps: Vec<i32> = Vec::new();
    dimension -= 2;
    if dimension <= 0 {
        return 0;
    }
    let mut i = dimension;
    while i > 4 {
        if dimension % i <= 1 {
            potential_gaps.push(i);
        }
        i -= 1;
    }
    if potential_gaps.is_empty() {
        return 4;
    }
    let target = dimension / 5;
    *potential_gaps
        .iter()
        .min_by_key(|g| (**g - target).abs())
        .expect("non-empty checked above")
}

pub struct Synthgrid;

impl Synthgrid {
    pub fn new() -> Self {
        Synthgrid
    }
}

impl Default for Synthgrid {
    fn default() -> Self {
        Synthgrid::new()
    }
}

impl Effect for Synthgrid {
    fn name(&self) -> &str {
        "synthgrid"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;
        let mut rng = Rng::new(0x9E37_79B9_7F4A_7C15 ^ (input.len() as u64).wrapping_mul(0x1000_0001B3));

        // --- gradients ---------------------------------------------------
        let grid_stops = [
            Color::from_hex("CC00CC").expect("valid hex"),
            Color::from_hex("ffffff").expect("valid hex"),
        ];
        let grid_gradient = Gradient::new(&grid_stops, GRADIENT_STEPS);
        let text_stops = [
            Color::from_hex("8A008A").expect("valid hex"),
            Color::from_hex("00D1FF").expect("valid hex"),
            Color::from_hex("FFFFFF").expect("valid hex"),
        ];
        let text_gradient = Gradient::new(&text_stops, GRADIENT_STEPS);

        // grid gradient is applied diagonally; text gradient vertically
        let grid_color_at = |coord: Coord| -> Option<Color> {
            let denom = ((width - 1) + (height - 1)).max(1) as f64;
            let frac = ((coord.column - 1) + (coord.row - 1)) as f64 / denom;
            grid_gradient.get_color_at_fraction(frac)
        };
        let text_color_at = |coord: Coord| -> Option<Color> {
            let frac = if height > 1 {
                (coord.row - 1) as f64 / (height - 1) as f64
            } else {
                1.0
            };
            text_gradient.get_color_at_fraction(frac)
        };

        let text_count = terminal.characters.len();

        // --- build dissolve scenes for the input text ---------------------
        for idx in 0..text_count {
            let coord = terminal.characters[idx].input_coord;
            let symbol = terminal.characters[idx].input_symbol;
            let final_color = text_color_at(coord);
            let colors = ColorPair::new(final_color, None);
            let frame_count = rng.gen_range(15, 30);
            let character = &mut terminal.characters[idx];
            character.is_visible = false;
            let scene = character.animation.new_scene("dissolve", false);
            for _ in 0..frame_count {
                let s = *rng.choice(&TEXT_GENERATION_SYMBOLS);
                scene.add_frame(s, 3, colors, false);
            }
            scene.add_frame(symbol, 1, colors, false);
        }

        // --- build grid lines ---------------------------------------------
        // Extra characters live in the arena after the text characters, so
        // they render on top of text at shared coordinates (Python layer 2).
        let mut grid_lines: Vec<GridLine> = Vec::new();
        let mut interior_rows: Vec<i32> = Vec::new();
        let mut interior_cols: Vec<i32> = Vec::new();

        let mut make_line = |terminal: &mut Terminal, origin: Coord, horizontal: bool| {
            let mut ids = Vec::new();
            if horizontal {
                for column in 1..=width {
                    let coord = Coord::new(column, origin.row);
                    let id = terminal.characters.len();
                    let mut ch = EffectCharacter::new(id, GRID_ROW_SYMBOL, coord);
                    ch.animation.current_visual = CharacterVisual::new(
                        GRID_ROW_SYMBOL,
                        false,
                        ColorPair::new(grid_color_at(coord), None),
                    );
                    ch.is_visible = false;
                    terminal.characters.push(ch);
                    ids.push(id);
                }
            } else {
                // vertical lines stop below the top row so the top horizontal
                // line owns the corner cells (mirrors range(bottom, top)).
                let top = (height - 1).max(1);
                for row in 1..=top {
                    let coord = Coord::new(origin.column, row);
                    let id = terminal.characters.len();
                    let mut ch = EffectCharacter::new(id, GRID_COLUMN_SYMBOL, coord);
                    ch.animation.current_visual = CharacterVisual::new(
                        GRID_COLUMN_SYMBOL,
                        false,
                        ColorPair::new(grid_color_at(coord), None),
                    );
                    ch.is_visible = false;
                    terminal.characters.push(ch);
                    ids.push(id);
                }
            }
            GridLine {
                ids,
                speed: if horizontal { 3 } else { 1 },
                shown: 0,
                hidden: 0,
            }
        };

        // border lines: bottom, top, left, right
        grid_lines.push(make_line(&mut terminal, Coord::new(1, 1), true));
        grid_lines.push(make_line(&mut terminal, Coord::new(1, height), true));
        grid_lines.push(make_line(&mut terminal, Coord::new(1, 1), false));
        grid_lines.push(make_line(&mut terminal, Coord::new(width, 1), false));

        // interior lines
        let row_gap = find_even_gap(height);
        if row_gap > 0 {
            let mut row = 1 + row_gap;
            while row < height {
                interior_rows.push(row);
                grid_lines.push(make_line(&mut terminal, Coord::new(1, row), true));
                row += row_gap;
            }
        }
        let column_gap = find_even_gap(width);
        if column_gap > 0 {
            let mut column = 1 + column_gap;
            while column < width {
                interior_cols.push(column);
                grid_lines.push(make_line(&mut terminal, Coord::new(column, 1), false));
                column += column_gap;
            }
        }

        // --- group text characters into grid blocks ------------------------
        let mut row_bounds = interior_rows.clone();
        row_bounds.push(height);
        let mut col_bounds = interior_cols.clone();
        col_bounds.push(width);

        let mut blocks: BTreeMap<(i32, i32), Vec<usize>> = BTreeMap::new();
        for idx in 0..text_count {
            let coord = terminal.characters[idx].input_coord;
            let block_row = *row_bounds
                .iter()
                .find(|&&r| coord.row <= r)
                .unwrap_or(&height);
            let block_col = *col_bounds
                .iter()
                .find(|&&c| coord.column <= c)
                .unwrap_or(&width);
            blocks.entry((block_row, block_col)).or_default().push(idx);
        }
        let mut pending_blocks: Vec<Vec<usize>> = blocks.into_values().collect();
        rng.shuffle(&mut pending_blocks);
        let max_active = (((pending_blocks.len() as f64) * MAX_ACTIVE_BLOCKS).round() as usize).max(1);

        // --- run the effect -------------------------------------------------
        let mut frames: Vec<String> = Vec::new();

        // Phase 1: expand the grid.
        while grid_lines.iter().any(|line| !line.is_extended()) && frames.len() < FRAME_BUDGET {
            for line in &mut grid_lines {
                line.extend(&mut terminal.characters);
            }
            terminal.tick();
            frames.push(terminal.get_formatted_output_string());
        }

        // Phase 2: dissolve text into the grid, a few blocks at a time.
        let mut active_blocks: Vec<Vec<usize>> = Vec::new();
        while (!pending_blocks.is_empty() || !active_blocks.is_empty())
            && frames.len() < FRAME_BUDGET
        {
            while active_blocks.len() < max_active && !pending_blocks.is_empty() {
                let block = pending_blocks.pop().expect("checked non-empty");
                for &idx in &block {
                    let character = &mut terminal.characters[idx];
                    character.is_visible = true;
                    character.animation.activate_scene("dissolve");
                }
                active_blocks.push(block);
            }
            terminal.tick();
            active_blocks.retain(|block| {
                block
                    .iter()
                    .any(|&idx| terminal.characters[idx].is_active())
            });
            frames.push(terminal.get_formatted_output_string());
        }

        // Phase 3: collapse the grid, leaving the finished text behind.
        while grid_lines.iter().any(|line| !line.is_collapsed()) && frames.len() < FRAME_BUDGET {
            for line in &mut grid_lines {
                line.collapse(&mut terminal.characters);
            }
            terminal.tick();
            frames.push(terminal.get_formatted_output_string());
        }

        // Final resting frame.
        frames.push(terminal.get_formatted_output_string());
        frames
    }
}
