//! Coord and geometry math, ported from utils/geometry.py. M0 ships Coord;
//! the full function set lands in M1.

/// 1-based canvas coordinate: column grows right, row grows UP (origin bottom-left).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Coord {
    pub column: i64,
    pub row: i64,
}

impl Coord {
    pub fn new(column: i64, row: i64) -> Self {
        Coord { column, row }
    }
}
