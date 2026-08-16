//! Canvas: a grid of styled cells, row 1 at the bottom (TTE convention).

use crate::engine::animation::CharacterVisual;
use crate::utils::geometry::Coord;

/// A rectangular grid of optional styled cells.
#[derive(Clone, Debug)]
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    cells: Vec<Option<CharacterVisual>>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        Canvas {
            width,
            height,
            cells: vec![None; width * height],
        }
    }

    /// True if the coord lies within the canvas (1-based, inclusive).
    pub fn coord_is_in_canvas(&self, coord: Coord) -> bool {
        coord.column >= 1
            && coord.column <= self.width as i32
            && coord.row >= 1
            && coord.row <= self.height as i32
    }

    fn index(&self, coord: Coord) -> usize {
        (coord.row as usize - 1) * self.width + (coord.column as usize - 1)
    }

    /// Center coordinate of the canvas.
    pub fn center(&self) -> Coord {
        Coord::new((self.width as i32 + 1) / 2, (self.height as i32 + 1) / 2)
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = None;
        }
    }

    pub fn set_cell(&mut self, coord: Coord, visual: CharacterVisual) {
        if self.coord_is_in_canvas(coord) {
            let idx = self.index(coord);
            self.cells[idx] = Some(visual);
        }
    }

    pub fn get_cell(&self, coord: Coord) -> Option<&CharacterVisual> {
        if self.coord_is_in_canvas(coord) {
            self.cells[self.index(coord)].as_ref()
        } else {
            None
        }
    }

    /// Render the canvas top-to-bottom into a printable frame string.
    pub fn to_frame_string(&self) -> String {
        let mut rows: Vec<String> = Vec::with_capacity(self.height);
        for row in (1..=self.height as i32).rev() {
            let mut line = String::new();
            for column in 1..=self.width as i32 {
                match self.get_cell(Coord::new(column, row)) {
                    Some(visual) => line.push_str(&visual.formatted()),
                    None => line.push(' '),
                }
            }
            rows.push(line);
        }
        rows.join("\n")
    }
}
