use crate::utils::round_half_even;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Coord {
    pub column: i32,
    pub row: i32,
}

impl Coord {
    pub fn new(column: i32, row: i32) -> Self {
        Self { column, row }
    }
}

impl std::fmt::Display for Coord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Coord({}, {})", self.column, self.row)
    }
}

pub fn find_length_of_line(start: Coord, end: Coord) -> f64 {
    let dc = f64::from(end.column - start.column);
    let dr = f64::from(end.row - start.row);
    dc.hypot(dr)
}

pub fn distance(a: Coord, b: Coord) -> f64 {
    find_length_of_line(a, b)
}

pub fn lerp_coord(start: Coord, end: Coord, t: f64) -> Coord {
    let column =
        f64::from(start.column) + (f64::from(end.column) - f64::from(start.column)) * t;
    let row = f64::from(start.row) + (f64::from(end.row) - f64::from(start.row)) * t;
    Coord {
        column: round_half_even(column) as i32,
        row: round_half_even(row) as i32,
    }
}

pub fn find_coord_on_bezier_curve(start: Coord, control: Coord, end: Coord, t: f64) -> Coord {
    let u = 1.0 - t;
    let column = u * u * f64::from(start.column)
        + 2.0 * u * t * f64::from(control.column)
        + t * t * f64::from(end.column);
    let row = u * u * f64::from(start.row)
        + 2.0 * u * t * f64::from(control.row)
        + t * t * f64::from(end.row);
    Coord {
        column: round_half_even(column) as i32,
        row: round_half_even(row) as i32,
    }
}

pub fn find_length_of_bezier_curve(start: Coord, control: Coord, end: Coord) -> f64 {
    const SAMPLES: usize = 50;
    let mut length = 0.0;
    let mut prev = start;
    for i in 1..=SAMPLES {
        let t = i as f64 / SAMPLES as f64;
        let point = find_coord_on_bezier_curve(start, control, end, t);
        length += find_length_of_line(prev, point);
        prev = point;
    }
    length
}

pub fn find_coord_at_distance(origin: Coord, target: Coord, distance: f64) -> Coord {
    let length = find_length_of_line(origin, target);
    if length == 0.0 {
        return origin;
    }
    lerp_coord(origin, target, distance / length)
}

pub fn find_coords_on_line(start: Coord, end: Coord) -> Vec<Coord> {
    let mut coords = Vec::new();
    let dx = (end.column - start.column).abs();
    let dy = (end.row - start.row).abs();
    let sx = if start.column < end.column { 1 } else { -1 };
    let sy = if start.row < end.row { 1 } else { -1 };
    let mut err = dx - dy;
    let mut x = start.column;
    let mut y = start.row;
    loop {
        coords.push(Coord { column: x, row: y });
        if x == end.column && y == end.row {
            break;
        }
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
    coords
}

pub fn find_coords_on_circle(
    center: Coord,
    radius: f64,
    num_points: usize,
    unique: bool,
) -> Vec<Coord> {
    if num_points == 0 {
        return Vec::new();
    }
    let mut coords = Vec::with_capacity(num_points);
    for i in 0..num_points {
        let angle = std::f64::consts::TAU * i as f64 / num_points as f64;
        let column = f64::from(center.column) + radius * angle.cos();
        let row = f64::from(center.row) + radius * angle.sin();
        let coord = Coord {
            column: round_half_even(column) as i32,
            row: round_half_even(row) as i32,
        };
        if unique && coords.contains(&coord) {
            continue;
        }
        coords.push(coord);
    }
    coords
}

pub fn find_coords_in_circle(center: Coord, radius: f64) -> Vec<Coord> {
    let mut coords = Vec::new();
    let r = radius.ceil() as i32;
    for row in (center.row - r)..=(center.row + r) {
        for column in (center.column - r)..=(center.column + r) {
            let coord = Coord { column, row };
            if find_length_of_line(center, coord) <= radius {
                coords.push(coord);
            }
        }
    }
    coords
}

pub fn find_coords_in_rect(origin: Coord, width: i32, height: i32) -> Vec<Coord> {
    let mut coords = Vec::new();
    if width <= 0 || height <= 0 {
        return coords;
    }
    for row in origin.row..origin.row + height {
        for column in origin.column..origin.column + width {
            coords.push(Coord { column, row });
        }
    }
    coords
}

pub fn find_coords_on_rect(origin: Coord, width: i32, height: i32) -> Vec<Coord> {
    let mut coords = Vec::new();
    if width <= 0 || height <= 0 {
        return coords;
    }
    let right = origin.column + width - 1;
    let top = origin.row + height - 1;
    for column in origin.column..=right {
        coords.push(Coord {
            column,
            row: origin.row,
        });
        if height > 1 {
            coords.push(Coord { column, row: top });
        }
    }
    if height > 2 {
        for row in (origin.row + 1)..top {
            coords.push(Coord {
                column: origin.column,
                row,
            });
            if width > 1 {
                coords.push(Coord { column: right, row });
            }
        }
    }
    coords
}
