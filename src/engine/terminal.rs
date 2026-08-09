//! Terminal: config, canvas assembly, character queries, renderer, tty output.
//! Ported from engine/terminal.py. Unlike upstream (which builds two Terminals
//! per run), this single Terminal owns both the simulation and the tty side.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::time::Instant;

use crate::engine::animation::ExistingColorHandling;
use crate::engine::canvas::{Anchor, Canvas};
use crate::engine::character::{CharId, EffectCharacter};
use crate::engine::error::EngineError;
use crate::engine::input::{ColorFrequency, Preprocessor};
use crate::utils::ansi;
use crate::utils::geometry::Coord;
use crate::utils::graphics::Color;
use crate::utils::rng::Rng;

#[derive(Debug, Clone)]
pub struct TerminalConfig {
    pub tab_width: i64,
    pub xterm_colors: bool,
    pub no_color: bool,
    pub terminal_background_color: Color,
    pub existing_color_handling: ExistingColorHandling,
    pub wrap_text: bool,
    pub frame_rate: i64,
    pub canvas_width: i64,
    pub canvas_height: i64,
    pub anchor_canvas: Anchor,
    pub anchor_text: Anchor,
    pub ignore_terminal_dimensions: bool,
    pub reuse_canvas: bool,
    pub no_eol: bool,
    pub no_restore_cursor: bool,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        TerminalConfig {
            tab_width: 4,
            xterm_colors: false,
            no_color: false,
            terminal_background_color: Color::from_hex("000000").unwrap(),
            existing_color_handling: ExistingColorHandling::Ignore,
            wrap_text: false,
            frame_rate: 60,
            canvas_width: -1,
            canvas_height: -1,
            anchor_canvas: Anchor::Sw,
            anchor_text: Anchor::Sw,
            ignore_terminal_dimensions: false,
            reuse_canvas: false,
            no_eol: false,
            no_restore_cursor: false,
        }
    }
}

/// CharacterSort (argutils.CharacterSort).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterSort {
    Random,
    TopToBottomLeftToRight,
    BottomToTopRightToLeft,
    BottomToTopLeftToRight,
    TopToBottomRightToLeft,
    OutsideRowToMiddle,
    MiddleRowToOutside,
}

/// CharacterGroup (argutils.CharacterGroup).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterGroup {
    ColumnLeftToRight,
    ColumnRightToLeft,
    RowTopToBottom,
    RowBottomToTop,
    DiagonalBottomLeftToTopRight,
    DiagonalTopRightToBottomLeft,
    DiagonalTopLeftToBottomRight,
    DiagonalBottomRightToTopLeft,
    CenterToOutside,
    OutsideToCenter,
}

/// ColorSort (argutils.ColorSort).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSort {
    LeastToMost,
    MostToLeast,
    Random,
}

/// Which character populations to include in a query (the four bool kwargs).
#[derive(Debug, Clone, Copy)]
pub struct CharacterFilter {
    pub input_chars: bool,
    pub inner_fill_chars: bool,
    pub outer_fill_chars: bool,
    pub added_chars: bool,
}

impl Default for CharacterFilter {
    fn default() -> Self {
        CharacterFilter { input_chars: true, inner_fill_chars: false, outer_fill_chars: false, added_chars: false }
    }
}

pub struct Terminal {
    pub config: TerminalConfig,
    pub canvas: Canvas,
    pub arena: Vec<EffectCharacter>,
    next_character_id: u32,
    pub input_colors_frequency: ColorFrequency,
    terminal_width: i64,
    terminal_height: i64,
    pub canvas_column_offset: i64,
    pub canvas_row_offset: i64,
    pub visible_top: i64,
    pub visible_bottom: i64,
    pub visible_right: i64,
    pub visible_left: i64,
    pub input_characters: Vec<CharId>,
    pub added_characters: Vec<CharId>,
    pub character_by_input_coord: HashMap<Coord, CharId>,
    pub inner_fill_characters: Vec<CharId>,
    pub outer_fill_characters: Vec<CharId>,
    visible_characters: HashSet<CharId>,
    pub terminal_state: Vec<String>,
    frame_rate: i64,
    last_time_printed: Instant,
}

impl Terminal {
    pub fn new(input_data: &str, config: TerminalConfig) -> Result<Self, EngineError> {
        let input_data = if input_data.is_empty() { "No Input." } else { input_data };
        let mut arena: Vec<EffectCharacter> = Vec::new();
        let mut next_character_id: u32 = 0;
        let mut input_colors_frequency = ColorFrequency::default();

        let preprocessed_lines = Preprocessor {
            arena: &mut arena,
            next_character_id: &mut next_character_id,
            input_colors_frequency: &mut input_colors_frequency,
            config: &config,
        }
        .preprocess(input_data)?;

        let (mut terminal_width, mut terminal_height) = get_terminal_dimensions();
        let (canvas_height, canvas_width) =
            get_canvas_dimensions(&config, &preprocessed_lines, terminal_width, terminal_height);
        let mut canvas = Canvas::new(canvas_height, canvas_width);

        let (canvas_column_offset, canvas_row_offset) = if !config.ignore_terminal_dimensions {
            calc_canvas_offsets(&config, &canvas, terminal_width, terminal_height)
        } else {
            terminal_width = canvas.right;
            terminal_height = canvas.top;
            (0, 0)
        };

        let visible_top = std::cmp::min(canvas.top + canvas_row_offset, terminal_height);
        let visible_bottom = std::cmp::max(canvas.bottom + canvas_row_offset, 1);
        let visible_right = std::cmp::min(canvas.right + canvas_column_offset, terminal_width);
        let visible_left = std::cmp::max(canvas.left + canvas_column_offset, 1);

        let input_characters = setup_input_characters(&config, &mut canvas, &mut arena, preprocessed_lines)?
            .into_iter()
            .filter(|&id| {
                let coord = arena[id.0 as usize].input_coord;
                coord.row <= canvas.top && coord.column <= canvas.right
            })
            .collect::<Vec<_>>();

        let mut character_by_input_coord: HashMap<Coord, CharId> = HashMap::new();
        for &id in &input_characters {
            character_by_input_coord.insert(arena[id.0 as usize].input_coord, id);
        }

        let frame_rate = config.frame_rate;
        let mut terminal = Terminal {
            config,
            canvas,
            arena,
            next_character_id,
            input_colors_frequency,
            terminal_width,
            terminal_height,
            canvas_column_offset,
            canvas_row_offset,
            visible_top,
            visible_bottom,
            visible_right,
            visible_left,
            input_characters,
            added_characters: Vec::new(),
            character_by_input_coord,
            inner_fill_characters: Vec::new(),
            outer_fill_characters: Vec::new(),
            visible_characters: HashSet::new(),
            terminal_state: Vec::new(),
            frame_rate,
            last_time_printed: Instant::now(),
        };
        terminal.make_fill_characters();
        terminal.setup_character_neighbors();
        terminal.update_terminal_state();
        Ok(terminal)
    }

    /// Terminal._make_fill_characters: row-major from (1,1), fresh space chars
    /// for unoccupied canvas coords, split inner/outer by the text bounds.
    fn make_fill_characters(&mut self) {
        for row in 1..=self.canvas.top {
            for column in 1..=self.canvas.right {
                let coord = Coord::new(column, row);
                if !self.character_by_input_coord.contains_key(&coord) {
                    let mut fill = EffectCharacter::new(self.next_character_id, " ", column, row);
                    fill.is_fill_character = true;
                    fill.animation.no_color = self.config.no_color;
                    fill.animation.use_xterm_colors = self.config.xterm_colors;
                    fill.animation.existing_color_handling = self.config.existing_color_handling;
                    fill.uses_input_preexisting_colors = false;
                    self.next_character_id += 1;
                    let id = CharId(self.arena.len() as u32);
                    self.arena.push(fill);
                    self.character_by_input_coord.insert(coord, id);
                    if self.canvas.text_left <= column
                        && column <= self.canvas.text_right
                        && self.canvas.text_bottom <= row
                        && row <= self.canvas.text_top
                    {
                        self.inner_fill_characters.push(id);
                    } else {
                        self.outer_fill_characters.push(id);
                    }
                }
            }
        }
    }

    fn setup_character_neighbors(&mut self) {
        let coords: Vec<(Coord, CharId)> = self.character_by_input_coord.iter().map(|(&c, &id)| (c, id)).collect();
        for (coord, id) in coords {
            let n = self.character_by_input_coord.get(&Coord::new(coord.column, coord.row + 1)).copied();
            let e = self.character_by_input_coord.get(&Coord::new(coord.column + 1, coord.row)).copied();
            let s = self.character_by_input_coord.get(&Coord::new(coord.column, coord.row - 1)).copied();
            let w = self.character_by_input_coord.get(&Coord::new(coord.column - 1, coord.row)).copied();
            let ch = &mut self.arena[id.0 as usize];
            ch.neighbors.north = n;
            ch.neighbors.east = e;
            ch.neighbors.south = s;
            ch.neighbors.west = w;
        }
    }

    /// Terminal.add_character: registered only in added_characters, not in
    /// character_by_input_coord or the neighbor map.
    pub fn add_character(&mut self, symbol: &str, coord: Coord) -> CharId {
        let mut ch = EffectCharacter::new(self.next_character_id, symbol, coord.column, coord.row);
        ch.animation.no_color = self.config.no_color;
        ch.animation.use_xterm_colors = self.config.xterm_colors;
        ch.animation.existing_color_handling = self.config.existing_color_handling;
        ch.uses_input_preexisting_colors = false;
        self.next_character_id += 1;
        let id = CharId(self.arena.len() as u32);
        self.arena.push(ch);
        self.added_characters.push(id);
        id
    }

    pub fn get_character_by_input_coord(&self, coord: Coord) -> Option<CharId> {
        self.character_by_input_coord.get(&coord).copied()
    }

    pub fn set_character_visibility(&mut self, id: CharId, is_visible: bool) {
        self.arena[id.0 as usize].is_visible = is_visible;
        if is_visible {
            self.visible_characters.insert(id);
        } else {
            self.visible_characters.remove(&id);
        }
    }

    /// Terminal.get_input_colors. Equal-count ties keep insertion order
    /// (Python's stable sort over dict keys).
    pub fn get_input_colors(&self, rng: &mut Rng, sort: ColorSort) -> Vec<Color> {
        let mut colors: Vec<(Color, i64)> = self.input_colors_frequency.0.clone();
        match sort {
            ColorSort::MostToLeast => {
                // Python: sorted(keys, key=count, reverse=True) — reverse of a
                // stable ascending sort reverses tie order too; replicate by
                // sorting descending with stable tie order = insertion order.
                colors.sort_by(|a, b| b.1.cmp(&a.1));
            }
            ColorSort::LeastToMost => {
                colors.sort_by(|a, b| a.1.cmp(&b.1));
            }
            ColorSort::Random => {
                rng.shuffle(&mut colors);
            }
        }
        colors.into_iter().map(|(c, _)| c).collect()
    }

    pub fn collect_characters(&self, filter: CharacterFilter) -> Vec<CharId> {
        let mut all: Vec<CharId> = Vec::new();
        if filter.input_chars {
            all.extend(&self.input_characters);
        }
        if filter.inner_fill_chars {
            all.extend(&self.inner_fill_characters);
        }
        if filter.outer_fill_chars {
            all.extend(&self.outer_fill_characters);
        }
        if filter.added_chars {
            all.extend(&self.added_characters);
        }
        all
    }

    /// Terminal.get_characters with all sort variants.
    pub fn get_characters(&self, rng: &mut Rng, filter: CharacterFilter, sort: CharacterSort) -> Vec<CharId> {
        let mut all = self.collect_characters(filter);
        // default sort: (-row, column), stable
        all.sort_by_key(|&id| {
            let c = self.arena[id.0 as usize].input_coord;
            (-c.row, c.column)
        });
        match sort {
            CharacterSort::Random => rng.shuffle(&mut all),
            CharacterSort::TopToBottomLeftToRight => {}
            CharacterSort::BottomToTopRightToLeft => all.reverse(),
            CharacterSort::BottomToTopLeftToRight | CharacterSort::TopToBottomRightToLeft => {
                all.sort_by_key(|&id| {
                    let c = self.arena[id.0 as usize].input_coord;
                    (c.row, c.column)
                });
                if sort == CharacterSort::TopToBottomRightToLeft {
                    all.reverse();
                }
            }
            CharacterSort::OutsideRowToMiddle | CharacterSort::MiddleRowToOutside => {
                // upstream: alternate pop(0)/pop(-1)
                let mut deque: std::collections::VecDeque<CharId> = all.into();
                let mut interleaved = Vec::with_capacity(deque.len());
                let mut from_front = true;
                while let Some(id) = if from_front { deque.pop_front() } else { deque.pop_back() } {
                    interleaved.push(id);
                    from_front = !from_front;
                }
                all = interleaved;
                if sort == CharacterSort::MiddleRowToOutside {
                    all.reverse();
                }
            }
        }
        all
    }

    /// Terminal.get_characters_grouped with all grouping variants.
    pub fn get_characters_grouped(&self, filter: CharacterFilter, grouping: CharacterGroup) -> Vec<Vec<CharId>> {
        let mut all = self.collect_characters(filter);
        all.sort_by_key(|&id| {
            let c = self.arena[id.0 as usize].input_coord;
            (c.row, c.column)
        });
        let coord = |id: &CharId| self.arena[id.0 as usize].input_coord;
        match grouping {
            CharacterGroup::ColumnLeftToRight | CharacterGroup::ColumnRightToLeft => {
                let mut columns: Vec<Vec<CharId>> = Vec::new();
                for column_index in 0..=self.canvas.right {
                    let in_column: Vec<CharId> =
                        all.iter().copied().filter(|id| coord(id).column == column_index).collect();
                    if !in_column.is_empty() {
                        columns.push(in_column);
                    }
                }
                if grouping == CharacterGroup::ColumnRightToLeft {
                    columns.reverse();
                }
                columns
            }
            CharacterGroup::RowBottomToTop | CharacterGroup::RowTopToBottom => {
                let mut rows: Vec<Vec<CharId>> = Vec::new();
                for row_index in 0..=self.canvas.top {
                    let in_row: Vec<CharId> = all.iter().copied().filter(|id| coord(id).row == row_index).collect();
                    if !in_row.is_empty() {
                        rows.push(in_row);
                    }
                }
                if grouping == CharacterGroup::RowTopToBottom {
                    rows.reverse();
                }
                rows
            }
            CharacterGroup::DiagonalBottomLeftToTopRight | CharacterGroup::DiagonalTopRightToBottomLeft => {
                let mut diagonals: Vec<Vec<CharId>> = Vec::new();
                for diagonal_index in 0..=(self.canvas.top + self.canvas.right) {
                    let in_diag: Vec<CharId> = all
                        .iter()
                        .copied()
                        .filter(|id| coord(id).row + coord(id).column == diagonal_index)
                        .collect();
                    if !in_diag.is_empty() {
                        diagonals.push(in_diag);
                    }
                }
                if grouping == CharacterGroup::DiagonalTopRightToBottomLeft {
                    diagonals.reverse();
                }
                diagonals
            }
            CharacterGroup::DiagonalTopLeftToBottomRight | CharacterGroup::DiagonalBottomRightToTopLeft => {
                let mut diagonals: Vec<Vec<CharId>> = Vec::new();
                for diagonal_index in (self.canvas.left - self.canvas.top)..=(self.canvas.right - self.canvas.bottom) {
                    let in_diag: Vec<CharId> = all
                        .iter()
                        .copied()
                        .filter(|id| coord(id).column - coord(id).row == diagonal_index)
                        .collect();
                    if !in_diag.is_empty() {
                        diagonals.push(in_diag);
                    }
                }
                if grouping == CharacterGroup::DiagonalBottomRightToTopLeft {
                    diagonals.reverse();
                }
                diagonals
            }
            CharacterGroup::CenterToOutside | CharacterGroup::OutsideToCenter => {
                // insertion-ordered distance map (Python dict semantics)
                let mut distances: Vec<(i64, Vec<CharId>)> = Vec::new();
                for &id in &all {
                    let c = coord(&id);
                    let distance = (c.column - self.canvas.text_center.column).abs()
                        + (c.row - self.canvas.text_center.row).abs();
                    if let Some(entry) = distances.iter_mut().find(|(d, _)| *d == distance) {
                        entry.1.push(id);
                    } else {
                        distances.push((distance, Vec::from([id])));
                    }
                }
                distances.sort_by_key(|&(d, _)| d);
                if grouping == CharacterGroup::OutsideToCenter {
                    distances.reverse();
                }
                distances.into_iter().map(|(_, group)| group).collect()
            }
        }
    }

    /// Terminal._update_terminal_state: full repaint, painter's algorithm by
    /// (layer, character_id) — the canonical deterministic order (plan.md §4.3).
    pub fn update_terminal_state(&mut self) {
        let width = self.visible_right.max(0) as usize;
        let height = self.visible_top.max(0) as usize;
        let mut rows: Vec<Vec<&str>> = vec![vec![" "; width]; height];
        let mut visible: Vec<CharId> = self.visible_characters.iter().copied().collect();
        visible.sort_by_key(|&id| {
            let ch = &self.arena[id.0 as usize];
            (ch.layer, ch.character_id)
        });
        for id in visible {
            let ch = &self.arena[id.0 as usize];
            let row = ch.motion.current_coord.row + self.canvas_row_offset;
            let column = ch.motion.current_coord.column + self.canvas_column_offset;
            if self.visible_bottom <= row
                && row <= self.visible_top
                && self.visible_left <= column
                && column <= self.visible_right
            {
                rows[(row - 1) as usize][(column - 1) as usize] =
                    &ch.animation.current_character_visual.formatted_symbol;
            }
        }
        self.terminal_state = rows.into_iter().map(|row| row.concat()).collect();
    }

    /// get_formatted_output_string: refresh + join top row first.
    pub fn get_formatted_output_string(&mut self) -> String {
        self.update_terminal_state();
        let mut out = String::new();
        for (i, row) in self.terminal_state.iter().rev().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(row);
        }
        out
    }

    // --- tty side (upstream's second Terminal instance) ---

    pub fn prep_canvas(&mut self, out: &mut impl Write) -> std::io::Result<()> {
        out.write_all(ansi::HIDE_CURSOR.as_bytes())?;
        if self.config.reuse_canvas {
            self.write_move_cursor_to_top(out)?;
        }
        for _ in 0..self.visible_top {
            let blank = " ".repeat(self.visible_right.max(0) as usize);
            out.write_all(blank.as_bytes())?;
            out.write_all(b"\n")?;
        }
        out.write_all(ansi::DEC_SAVE_CURSOR.as_bytes())?;
        Ok(())
    }

    pub fn restore_cursor(&self, out: &mut impl Write, end_symbol: &str) -> std::io::Result<()> {
        let end_symbol = if self.config.no_eol { "" } else { end_symbol };
        if !self.config.no_restore_cursor {
            out.write_all(ansi::SHOW_CURSOR.as_bytes())?;
        }
        out.write_all(end_symbol.as_bytes())?;
        Ok(())
    }

    pub fn print_frame(&mut self, out: &mut impl Write, output_string: &str) -> std::io::Result<()> {
        self.write_move_cursor_to_top(out)?;
        out.write_all(output_string.as_bytes())?;
        out.flush()
    }

    fn write_move_cursor_to_top(&self, out: &mut impl Write) -> std::io::Result<()> {
        out.write_all(ansi::DEC_RESTORE_CURSOR.as_bytes())?;
        out.write_all(ansi::DEC_SAVE_CURSOR.as_bytes())?;
        out.write_all(ansi::move_cursor_up(self.visible_top.max(0) as usize).as_bytes())?;
        Ok(())
    }

    /// Terminal.enforce_framerate: sleep off the remainder; timestamp taken
    /// AFTER the sleep (drift accumulates, faithfully).
    pub fn enforce_framerate(&mut self) {
        if self.frame_rate == 0 {
            return;
        }
        let frame_delay = 1.0 / self.frame_rate as f64;
        let elapsed = self.last_time_printed.elapsed().as_secs_f64();
        if elapsed < frame_delay {
            std::thread::sleep(std::time::Duration::from_secs_f64(frame_delay - elapsed));
        }
        self.last_time_printed = Instant::now();
    }
}

/// shutil.get_terminal_size semantics: COLUMNS/LINES env vars win; else query
/// the tty; on failure (80, 24).
fn get_terminal_dimensions() -> (i64, i64) {
    let env_dim = |name: &str| -> Option<i64> {
        std::env::var(name).ok()?.parse::<i64>().ok()
    };
    let columns = env_dim("COLUMNS");
    let lines = env_dim("LINES");
    if let (Some(c), Some(l)) = (columns, lines) {
        return (c, l);
    }
    match terminal_size::terminal_size() {
        Some((terminal_size::Width(w), terminal_size::Height(h))) => {
            (columns.unwrap_or(w as i64), lines.unwrap_or(h as i64))
        }
        None => (columns.unwrap_or(80), lines.unwrap_or(24)),
    }
}

/// Terminal._get_canvas_dimensions -> (height, width).
fn get_canvas_dimensions(
    config: &TerminalConfig,
    preprocessed_lines: &[Vec<CharId>],
    terminal_width: i64,
    terminal_height: i64,
) -> (i64, i64) {
    let canvas_width = if config.canvas_width > 0 {
        config.canvas_width
    } else if config.canvas_width == 0 {
        terminal_width
    } else {
        let input_width = preprocessed_lines.iter().map(|l| l.len() as i64).max().unwrap_or(0);
        if config.ignore_terminal_dimensions {
            input_width
        } else {
            std::cmp::min(terminal_width, input_width)
        }
    };
    let canvas_height = if config.canvas_height > 0 {
        config.canvas_height
    } else if config.canvas_height == 0 {
        terminal_height
    } else {
        let input_height = preprocessed_lines.len() as i64;
        if config.ignore_terminal_dimensions {
            input_height
        } else if config.wrap_text {
            std::cmp::min(wrapped_line_count(preprocessed_lines, canvas_width), terminal_height)
        } else {
            std::cmp::min(terminal_height, input_height)
        }
    };
    (canvas_height, canvas_width)
}

fn wrapped_line_count(lines: &[Vec<CharId>], width: i64) -> i64 {
    let mut count: i64 = 0;
    for line in lines {
        let mut remaining = line.len() as i64;
        while remaining > width {
            count += 1;
            remaining -= width;
        }
        count += 1;
    }
    count
}

/// Terminal._wrap_lines.
fn wrap_lines(lines: Vec<Vec<CharId>>, width: i64) -> Vec<Vec<CharId>> {
    let mut wrapped: Vec<Vec<CharId>> = Vec::new();
    for line in lines {
        let mut current = line;
        while current.len() as i64 > width {
            let rest = current.split_off(width as usize);
            wrapped.push(current);
            current = rest;
        }
        wrapped.push(current);
    }
    wrapped
}

fn calc_canvas_offsets(
    config: &TerminalConfig,
    canvas: &Canvas,
    terminal_width: i64,
    terminal_height: i64,
) -> (i64, i64) {
    use crate::engine::canvas::Anchor::*;
    use crate::utils::pycompat::floor_div;
    let mut column_offset = 0;
    let mut row_offset = 0;
    match config.anchor_canvas {
        S | N | C => column_offset = floor_div(terminal_width, 2) - floor_div(canvas.width, 2),
        Se | E | Ne => column_offset = terminal_width - canvas.width,
        _ => {}
    }
    match config.anchor_canvas {
        W | E | C => row_offset = floor_div(terminal_height, 2) - floor_div(canvas.height, 2),
        Nw | N | Ne => row_offset = terminal_height - canvas.height,
        _ => {}
    }
    (column_offset, row_offset)
}

/// Terminal._setup_input_characters: wrap, assign 1-based bottom-up coords,
/// drop plain spaces (they become fill), anchor, and keep in-canvas chars.
fn setup_input_characters(
    config: &TerminalConfig,
    canvas: &mut Canvas,
    arena: &mut Vec<EffectCharacter>,
    preprocessed_lines: Vec<Vec<CharId>>,
) -> Result<Vec<CharId>, EngineError> {
    let formatted_lines = if config.wrap_text {
        wrap_lines(preprocessed_lines, canvas.right)
    } else {
        preprocessed_lines
    };
    let input_height = formatted_lines.len() as i64;
    let mut input_characters: Vec<CharId> = Vec::new();
    for (row, line) in formatted_lines.iter().enumerate() {
        for (column0, &id) in line.iter().enumerate() {
            let column = column0 as i64 + 1;
            let ch = &mut arena[id.0 as usize];
            ch.input_coord = Coord::new(column, input_height - row as i64);
            if ch.input_symbol != " "
                || ch.animation.input_fg_color.is_some()
                || ch.animation.input_bg_color.is_some()
            {
                input_characters.push(id);
            }
        }
    }
    canvas
        .anchor_text(arena, input_characters, config.anchor_text)
        .map_err(EngineError::Other)
}
