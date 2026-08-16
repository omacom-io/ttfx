//! Coordinate type and geometric helpers (mirrors
//! terminaltexteffects/utils/geometry.py).

/// A discrete canvas coordinate. `column` is x, `row` is y, both origin at (0,0)
/// top-left, matching the Python `Coord` namedtuple's field order semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Coord {
    pub column: i32,
    pub row: i32,
}

impl Coord {
    pub fn new(column: i32, row: i32) -> Self {
        Coord { column, row }
    }
}

/// Euclidean distance between two coordinates.
pub fn distance(a: Coord, b: Coord) -> f64 {
    let dx = (b.column - a.column) as f64;
    let dy = (b.row - a.row) as f64;
    (dx * dx + dy * dy).sqrt()
}

/// Length of a straight line segment between two coordinates.
pub fn find_length_of_line(start: Coord, end: Coord) -> f64 {
    distance(start, end)
}

/// Approximate arc length of a quadratic bezier curve by sampling.
pub fn find_length_of_bezier_curve(start: Coord, control: Coord, end: Coord) -> f64 {
    const SAMPLES: usize = 20;
    let mut length = 0.0;
    let mut prev = bezier_point(start, control, end, 0.0);
    for i in 1..=SAMPLES {
        let t = i as f64 / SAMPLES as f64;
        let pt = bezier_point(start, control, end, t);
        let dx = pt.0 - prev.0;
        let dy = pt.1 - prev.1;
        length += (dx * dx + dy * dy).sqrt();
        prev = pt;
    }
    length
}

/// Point on a quadratic bezier curve at parameter `t` in [0, 1], returned as
/// floating point (column, row) since sub-cell precision is needed for motion.
pub fn bezier_point(start: Coord, control: Coord, end: Coord, t: f64) -> (f64, f64) {
    let one_minus_t = 1.0 - t;
    let x = one_minus_t * one_minus_t * start.column as f64
        + 2.0 * one_minus_t * t * control.column as f64
        + t * t * end.column as f64;
    let y = one_minus_t * one_minus_t * start.row as f64
        + 2.0 * one_minus_t * t * control.row as f64
        + t * t * end.row as f64;
    (x, y)
}

/// Linear interpolation between two coordinates, returned as floating point.
pub fn lerp(a: Coord, b: Coord, t: f64) -> (f64, f64) {
    let x = a.column as f64 + (b.column as f64 - a.column as f64) * t;
    let y = a.row as f64 + (b.row as f64 - a.row as f64) * t;
    (x, y)
}
