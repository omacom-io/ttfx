//! Waypoint, Segment, Path, Motion (mirrors
//! terminaltexteffects/engine/motion.py, simplified: linear/bezier
//! interpolation only, no speed-curve holdover/tolerance handling yet).

use std::collections::HashMap;

use crate::utils::easing::EasingFunction;
use crate::utils::geometry::{self, Coord};

#[derive(Debug, Clone, Copy)]
pub struct Waypoint {
    pub coord: Coord,
}

impl Waypoint {
    pub fn new(coord: Coord) -> Self {
        Waypoint { coord }
    }
}

/// One leg of a path between two waypoints, with precomputed length.
#[derive(Debug, Clone, Copy)]
pub struct Segment {
    pub start: Waypoint,
    pub end: Waypoint,
    pub distance: f64,
}

impl Segment {
    pub fn new(start: Waypoint, end: Waypoint) -> Self {
        let distance = geometry::find_length_of_line(start.coord, end.coord);
        Segment { start, end, distance }
    }
}

/// A named sequence of segments a character can travel along.
#[derive(Debug, Clone)]
pub struct Path {
    pub id: String,
    pub segments: Vec<Segment>,
    pub speed: f64,
    pub loop_path: bool,
    pub ease: Option<EasingFunction>,
}

impl Path {
    pub fn new(id: impl Into<String>, speed: f64) -> Self {
        Path { id: id.into(), segments: Vec::new(), speed: speed.max(0.001), loop_path: false, ease: None }
    }

    pub fn add_waypoint(&mut self, coord: Coord) {
        let end = Waypoint::new(coord);
        if let Some(last) = self.segments.last() {
            let start = last.end;
            self.segments.push(Segment::new(start, end));
        } else {
            // First waypoint added has no prior point; store as a
            // zero-length segment anchor so `total_distance` starts from it.
            self.segments.push(Segment::new(end, end));
        }
    }

    pub fn total_distance(&self) -> f64 {
        self.segments.iter().map(|s| s.distance).sum()
    }
}

/// Per-character motion state: current position, registered paths, and
/// progress along the active path.
#[derive(Debug, Clone)]
pub struct Motion {
    pub current_pos: (f64, f64),
    pub current_coord: Coord,
    pub paths: HashMap<String, Path>,
    pub active_path_id: Option<String>,
    distance_traveled: f64,
}

impl Motion {
    pub fn new(start: Coord) -> Self {
        Motion {
            current_pos: (start.column as f64, start.row as f64),
            current_coord: start,
            paths: HashMap::new(),
            active_path_id: None,
            distance_traveled: 0.0,
        }
    }

    pub fn add_path(&mut self, path: Path) {
        self.paths.insert(path.id.clone(), path);
    }

    /// Activate a registered path by id, resetting progress, mirroring
    /// `Motion.activate_path`.
    pub fn activate_path(&mut self, path_id: &str) {
        if self.paths.contains_key(path_id) {
            self.active_path_id = Some(path_id.to_string());
            self.distance_traveled = 0.0;
        }
    }

    /// Advance along the active path by its speed for one tick, mirroring
    /// the per-tick displacement performed by `Motion.move`.
    pub fn step(&mut self) {
        let Some(path_id) = self.active_path_id.clone() else { return };
        let Some(path) = self.paths.get(&path_id) else { return };
        let total = path.total_distance();
        if total <= 0.0 {
            return;
        }

        self.distance_traveled += path.speed;
        if self.distance_traveled >= total {
            if path.loop_path {
                self.distance_traveled %= total;
            } else {
                self.distance_traveled = total;
            }
        }

        // Walk segments to find which one contains distance_traveled.
        let mut remaining = self.distance_traveled;
        for segment in &path.segments {
            if remaining <= segment.distance || segment.distance == 0.0 {
                let t = if segment.distance > 0.0 { remaining / segment.distance } else { 1.0 };
                let t = t.clamp(0.0, 1.0);
                let eased_t = path.ease.map(|f| f(t)).unwrap_or(t);
                let (x, y) = geometry::lerp(segment.start.coord, segment.end.coord, eased_t);
                self.current_pos = (x, y);
                self.current_coord = Coord::new(x.round() as i32, y.round() as i32);
                return;
            }
            remaining -= segment.distance;
        }

        // Traveled the full path; snap to final waypoint.
        if let Some(last) = path.segments.last() {
            self.current_pos = (last.end.coord.column as f64, last.end.coord.row as f64);
            self.current_coord = last.end.coord;
        }
    }
}
