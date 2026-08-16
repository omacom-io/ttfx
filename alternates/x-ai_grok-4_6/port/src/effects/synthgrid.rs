use std::collections::{HashMap, VecDeque};

use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::canvas::Canvas;
use crate::engine::character::{CharacterId, EffectCharacter};
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{find_length_of_line, lerp_coord, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient, GradientDirection};
use crate::utils::round_half_even;

const GRID_ROW_SYMBOL: &str = "─";
const GRID_COL_SYMBOL: &str = "│";
const GEN_SYMBOLS: [&str; 4] = ["░", "▒", "▓", "█"];
const MAX_ACTIVE_BLOCKS: f64 = 0.1;
const PATH_SPEED: f64 = 0.35;
const EXPAND_DELAY: i32 = 10;
const FINAL_HOLD: i32 = 20;
const FRAME_CAP: usize = 20_000;

pub struct Synthgrid;

impl Synthgrid {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Synthgrid {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Synthgrid {
    fn name(&self) -> &str {
        "synthgrid"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let term = Terminal::from_input(input, TerminalConfig::default());
        let width = term.canvas.width;
        let height = term.canvas.height;
        if width == 0 || height == 0 {
            return vec![String::new()];
        }

        let left = term.canvas.left;
        let right = term.canvas.right;
        let top = term.canvas.top;
        let bottom = term.canvas.bottom;

        let grid_stops = [
            Color::from_hex("8A008A").unwrap_or(Color::rgb(0x8a, 0x00, 0x8a)),
            Color::from_hex("00D1FF").unwrap_or(Color::rgb(0x00, 0xd1, 0xff)),
            Color::from_hex("FFFFFF").unwrap_or(Color::rgb(0xff, 0xff, 0xff)),
        ];
        let text_stops = grid_stops;
        let grid_gradient = Gradient::new(&grid_stops, 12);
        let text_gradient = Gradient::new(&text_stops, 12);

        let mut rng = Rng::from_input(input);

        let mut chars: Vec<InputChar> = term
            .get_characters()
            .iter()
            .map(|ch| InputChar {
                coord: ch.input_coord,
                symbol: ch.input_symbol.clone(),
                color: Color::rgb(255, 255, 255),
                visible: false,
                gen: Vec::new(),
                gen_idx: 0,
                playing: false,
            })
            .collect();

        if !chars.is_empty() {
            let text_left = chars.iter().map(|c| c.coord.column).min().unwrap_or(left);
            let text_right = chars.iter().map(|c| c.coord.column).max().unwrap_or(right);
            let text_bottom = chars.iter().map(|c| c.coord.row).min().unwrap_or(bottom);
            let text_top = chars.iter().map(|c| c.coord.row).max().unwrap_or(top);
            for ch in &mut chars {
                ch.color = mapped_color(
                    &text_gradient,
                    ch.coord,
                    text_left,
                    text_right,
                    text_bottom,
                    text_top,
                    GradientDirection::Vertical,
                );
            }
        }

        let row_gap = find_even_gap(top);
        let col_gap = find_even_gap(right);

        let mut row_indexes = Vec::new();
        let mut row = bottom;
        while row < top {
            row_indexes.push(row);
            row += row_gap;
        }
        row_indexes.push(top);

        let mut col_indexes = Vec::new();
        let mut col = left;
        while col < right {
            col_indexes.push(col);
            col += col_gap;
        }
        col_indexes.push(right);

        let mut grid_lines: Vec<GridLine> = Vec::new();
        for &row_index in &row_indexes {
            if row_index == bottom {
                continue;
            }
            grid_lines.push(GridLine::horizontal(
                left,
                right,
                row_index,
                &grid_gradient,
                bottom,
                top,
            ));
        }
        for &col_index in &col_indexes {
            if col_index == left {
                continue;
            }
            grid_lines.push(GridLine::vertical(
                bottom,
                top,
                col_index,
                &grid_gradient,
                left,
                right,
            ));
        }

        let mut by_coord: HashMap<Coord, usize> = HashMap::new();
        for (idx, ch) in chars.iter().enumerate() {
            by_coord.insert(ch.coord, idx);
        }

        let mut pending: VecDeque<Vec<usize>> = VecDeque::new();
        for &row_index in row_indexes.iter().skip(1) {
            for &col_index in col_indexes.iter().skip(1) {
                let mut group = Vec::new();
                let row_start = row_index - row_gap;
                let col_start = col_index - col_gap;
                let mut r = row_start;
                while r < row_index {
                    let mut c = col_start;
                    while c < col_index {
                        if c >= left && c <= right && r >= bottom && r <= top {
                            if let Some(&idx) = by_coord.get(&Coord::new(c, r)) {
                                group.push(idx);
                            }
                        }
                        c += 1;
                    }
                    r += 1;
                }
                if !group.is_empty() {
                    pending.push_back(group);
                }
            }
        }

        for group in pending.iter() {
            for &idx in group {
                let n = rng.randint(15, 30);
                let mut frames = Vec::with_capacity(n as usize * 3 + 1);
                for _ in 0..n {
                    let sym = rng.choice(&GEN_SYMBOLS);
                    for _ in 0..3 {
                        frames.push((*sym).to_string());
                    }
                }
                frames.push(chars[idx].symbol.clone());
                chars[idx].gen = frames;
            }
        }
        rng.shuffle_deque(&mut pending);

        let mut active: Vec<Vec<usize>> = Vec::new();
        let mut expanding = true;
        let mut collapse_started = false;
        let mut delay = 0;
        let mut hold = 0;
        let mut out = Vec::new();

        for _ in 0..FRAME_CAP {
            let grid_active = grid_lines.iter().any(|line| line.is_active());

            if expanding {
                if !grid_active {
                    expanding = false;
                    delay = EXPAND_DELAY;
                }
            } else if hold == 0 {
                if delay > 0 {
                    delay -= 1;
                } else if !pending.is_empty() && active.is_empty() {
                    let n = ((pending.len() as f64 * MAX_ACTIVE_BLOCKS) as usize) + 1;
                    let n = n.min(pending.len());
                    for _ in 0..n {
                        if let Some(group) = pending.pop_front() {
                            for &idx in &group {
                                chars[idx].visible = true;
                                chars[idx].gen_idx = 0;
                                chars[idx].playing = !chars[idx].gen.is_empty();
                            }
                            active.push(group);
                        }
                    }
                } else if pending.is_empty() && active.is_empty() && !collapse_started {
                    collapse_started = true;
                    for line in &mut grid_lines {
                        line.collapse();
                    }
                } else if collapse_started && !grid_active {
                    hold = 1;
                }

                active.retain(|group| group.iter().any(|&idx| chars[idx].playing));
            }

            out.push(render_frame(width, height, &chars, &grid_lines));

            if hold > 0 {
                hold += 1;
                if hold > FINAL_HOLD {
                    break;
                }
                continue;
            }

            for line in &mut grid_lines {
                line.step();
            }
            for ch in &mut chars {
                if ch.visible && ch.playing {
                    if ch.gen_idx + 1 < ch.gen.len() {
                        ch.gen_idx += 1;
                    } else {
                        ch.playing = false;
                    }
                }
            }
        }

        if out.is_empty() {
            out.push(render_frame(width, height, &chars, &grid_lines));
        }
        out
    }
}

fn find_even_gap(dimension: i32) -> i32 {
    if dimension <= 2 {
        return 1;
    }
    let dim_20 = round_half_even(f64::from(dimension) * 0.2) as i32;
    let mut even_gap = if dim_20 % 2 == 0 { dim_20 } else { dim_20 + 1 };
    if even_gap < 2 {
        even_gap = 2;
    }
    while even_gap > 1 && dimension % even_gap != 1 {
        even_gap -= 2;
    }
    if even_gap < 1 {
        1
    } else {
        even_gap
    }
}

fn out_quad(t: f64) -> f64 {
    let u = 1.0 - t;
    1.0 - u * u
}

fn mapped_color(
    gradient: &Gradient,
    coord: Coord,
    min_col: i32,
    max_col: i32,
    min_row: i32,
    max_row: i32,
    direction: GradientDirection,
) -> Color {
    let dw = f64::from((max_col - min_col).max(1));
    let dh = f64::from((max_row - min_row).max(1));
    let x = f64::from(coord.column - min_col);
    let y = f64::from(coord.row - min_row);
    let t = match direction {
        GradientDirection::Horizontal => x / dw,
        GradientDirection::Vertical => y / dh,
        GradientDirection::Diagonal => (x / dw + y / dh) * 0.5,
        GradientDirection::Radial | GradientDirection::Center => {
            let cx = dw * 0.5;
            let cy = dh * 0.5;
            let max_d = cx.hypot(cy).max(1.0);
            (x - cx).hypot(y - cy) / max_d
        }
    }
    .clamp(0.0, 1.0);
    gradient
        .mapped_color(t)
        .unwrap_or(Color::rgb(255, 255, 255))
}

fn make_visual(symbol: &str, color: Color) -> CharacterVisual {
    let mut ch = EffectCharacter::new(CharacterId(0), symbol, Coord::new(0, 0));
    ch.animation
        .set_appearance(symbol, Some(ColorPair::fg(color)));
    ch.animation.current_character_visual.clone()
}

fn render_frame(
    width: usize,
    height: usize,
    chars: &[InputChar],
    grid_lines: &[GridLine],
) -> String {
    let mut canvas = Canvas::new(width, height);
    for ch in chars {
        if !ch.visible {
            continue;
        }
        let symbol = ch
            .gen
            .get(ch.gen_idx)
            .map(String::as_str)
            .unwrap_or(ch.symbol.as_str());
        canvas.put(ch.coord, make_visual(symbol, ch.color));
    }
    for line in grid_lines {
        for cell in &line.cells {
            if cell.visible {
                canvas.put(cell.current_coord(), make_visual(&cell.symbol, cell.color));
            }
        }
    }
    canvas.render()
}

struct InputChar {
    coord: Coord,
    symbol: String,
    color: Color,
    visible: bool,
    gen: Vec<String>,
    gen_idx: usize,
    playing: bool,
}

struct GridCell {
    origin: Coord,
    dest: Coord,
    progress: f64,
    length: f64,
    visible: bool,
    symbol: String,
    color: Color,
    collapsing: bool,
}

impl GridCell {
    fn current_coord(&self) -> Coord {
        let t = out_quad(self.progress.clamp(0.0, 1.0));
        if self.collapsing {
            lerp_coord(self.dest, self.origin, t)
        } else {
            lerp_coord(self.origin, self.dest, t)
        }
    }

    fn step(&mut self) {
        if self.progress >= 1.0 {
            return;
        }
        if self.length < 1e-9 {
            self.progress = 1.0;
            return;
        }
        self.progress = (self.progress + PATH_SPEED / self.length).min(1.0);
    }

    fn is_active(&self) -> bool {
        self.progress < 1.0
    }

    fn collapse(&mut self) {
        if self.collapsing {
            return;
        }
        self.collapsing = true;
        self.progress = 0.0;
    }
}

struct GridLine {
    cells: Vec<GridCell>,
}

impl GridLine {
    fn horizontal(
        left: i32,
        right: i32,
        row: i32,
        gradient: &Gradient,
        canvas_bottom: i32,
        canvas_top: i32,
    ) -> Self {
        let origin = Coord::new(right, row);
        let mut cells = Vec::new();
        let mut count = 0;
        for column in left..=right {
            if count % 2 == 0 {
                let dest = Coord::new(column, row);
                cells.push(GridCell {
                    origin,
                    dest,
                    progress: 0.0,
                    length: find_length_of_line(origin, dest),
                    visible: true,
                    symbol: GRID_ROW_SYMBOL.to_string(),
                    color: mapped_color(
                        gradient,
                        dest,
                        left,
                        right,
                        canvas_bottom,
                        canvas_top,
                        GradientDirection::Diagonal,
                    ),
                    collapsing: false,
                });
            }
            count += 1;
        }
        Self { cells }
    }

    fn vertical(
        bottom: i32,
        top: i32,
        column: i32,
        gradient: &Gradient,
        canvas_left: i32,
        canvas_right: i32,
    ) -> Self {
        let origin = Coord::new(column, top);
        let mut cells = Vec::new();
        let mut count = 0;
        let mut row = bottom;
        while row < top {
            if count % 2 == 0 {
                let dest = Coord::new(column, row);
                cells.push(GridCell {
                    origin,
                    dest,
                    progress: 0.0,
                    length: find_length_of_line(origin, dest),
                    visible: true,
                    symbol: GRID_COL_SYMBOL.to_string(),
                    color: mapped_color(
                        gradient,
                        dest,
                        canvas_left,
                        canvas_right,
                        bottom,
                        top,
                        GradientDirection::Diagonal,
                    ),
                    collapsing: false,
                });
            }
            count += 1;
            row += 1;
        }
        Self { cells }
    }

    fn step(&mut self) {
        for cell in &mut self.cells {
            cell.step();
        }
    }

    fn collapse(&mut self) {
        for cell in &mut self.cells {
            cell.collapse();
        }
    }

    fn is_active(&self) -> bool {
        self.cells.iter().any(GridCell::is_active)
    }
}

struct Rng(u32);

impl Rng {
    fn from_input(input: &str) -> Self {
        let mut seed = 0x9e37_79b9u32;
        for b in input.as_bytes() {
            seed = seed.wrapping_mul(16_777_619) ^ u32::from(*b);
        }
        if seed == 0 {
            seed = 1;
        }
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }

    fn randint(&mut self, lo: u32, hi: u32) -> u32 {
        let span = hi.saturating_sub(lo).saturating_add(1);
        if span == 0 {
            return lo;
        }
        lo + self.next_u32() % span
    }

    fn choice<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.next_u32() as usize % items.len()]
    }

    fn shuffle_deque<T>(&mut self, items: &mut VecDeque<T>) {
        let mut tmp: Vec<T> = items.drain(..).collect();
        for i in (1..tmp.len()).rev() {
            let j = self.next_u32() as usize % (i + 1);
            tmp.swap(i, j);
        }
        items.extend(tmp);
    }
}
