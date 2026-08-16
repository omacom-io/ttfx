use crate::engine::animation::CharacterVisual;
use crate::utils::geometry::Coord;

#[derive(Clone, Debug)]
pub struct Cell {
    pub visual: CharacterVisual,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            visual: CharacterVisual::new(" ", None),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
    cells: Vec<Cell>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        let width = width;
        let height = height;
        let cells = vec![Cell::default(); width.saturating_mul(height)];
        Self {
            width,
            height,
            left: 1,
            right: width as i32,
            top: height as i32,
            bottom: 1,
            cells,
        }
    }

    pub fn center(&self) -> Coord {
        Coord {
            column: crate::utils::round_half_even(f64::from(self.left + self.right) / 2.0) as i32,
            row: crate::utils::round_half_even(f64::from(self.top + self.bottom) / 2.0) as i32,
        }
    }

    pub fn contains(&self, coord: Coord) -> bool {
        coord.column >= self.left
            && coord.column <= self.right
            && coord.row >= self.bottom
            && coord.row <= self.top
            && self.width > 0
            && self.height > 0
    }

    fn idx(&self, coord: Coord) -> Option<usize> {
        if !self.contains(coord) {
            return None;
        }
        let x = (coord.column - self.left) as usize;
        let y = (coord.row - self.bottom) as usize;
        Some(y * self.width + x)
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = Cell::default();
        }
    }

    pub fn put(&mut self, coord: Coord, visual: CharacterVisual) {
        if let Some(idx) = self.idx(coord) {
            self.cells[idx] = Cell { visual };
        }
    }

    pub fn get(&self, coord: Coord) -> Option<&Cell> {
        self.idx(coord).and_then(|idx| self.cells.get(idx))
    }

    pub fn get_mut(&mut self, coord: Coord) -> Option<&mut Cell> {
        self.idx(coord).and_then(|idx| self.cells.get_mut(idx))
    }

    pub fn render(&self) -> String {
        if self.width == 0 || self.height == 0 {
            return String::new();
        }
        let mut out = String::with_capacity((self.width + 1) * self.height);
        for row in (self.bottom..=self.top).rev() {
            for column in self.left..=self.right {
                if let Some(cell) = self.get(Coord { column, row }) {
                    out.push_str(&cell.visual.format_symbol());
                } else {
                    out.push(' ');
                }
            }
            out.push('\n');
        }
        out
    }
}
