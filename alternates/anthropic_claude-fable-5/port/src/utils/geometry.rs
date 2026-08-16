//! Coordinates and line math.

/// A 1-based canvas coordinate; row 1 is the bottom row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Coord {
    pub column: i32,
    pub row: i32,
}

impl Coord {
    pub fn new(column: i32, row: i32) -> Self {
        Coord { column, row }
    }
}

/// Euclidean length of the line between two coords.
pub fn find_length_of_line(start: Coord, end: Coord) -> f64 {
    let dx = (end.column - start.column) as f64;
    let dy = (end.row - start.row) as f64;
    (dx * dx + dy * dy).sqrt()
}

/// Point at fraction `t` (0..=1) along the line from `start` to `end`,
/// quantized to the character grid.
pub fn interpolate(start: Coord, end: Coord, t: f64) -> Coord {
    let column = start.column as f64 + (end.column - start.column) as f64 * t;
    let row = start.row as f64 + (end.row - start.row) as f64 * t;
    Coord::new(column.round() as i32, row.round() as i32)
}
