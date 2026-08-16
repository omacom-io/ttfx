//! Waypoints, segments, paths and per-character motion.

use std::collections::HashMap;

use crate::utils::easing::EasingFn;
use crate::utils::geometry::{find_length_of_line, interpolate, Coord};

/// A named coordinate along a path.
#[derive(Clone, Debug, PartialEq)]
pub struct Waypoint {
    pub waypoint_id: String,
    pub coord: Coord,
}

impl Waypoint {
    pub fn new(waypoint_id: &str, coord: Coord) -> Self {
        Waypoint {
            waypoint_id: waypoint_id.to_string(),
            coord,
        }
    }
}

/// A straight run between two waypoints with its cached length.
#[derive(Clone, Debug)]
pub struct Segment {
    pub start: Waypoint,
    pub end: Waypoint,
    pub distance: f64,
}

/// A polyline traversed at a fixed speed with optional easing.
#[derive(Clone, Debug)]
pub struct Path {
    pub path_id: String,
    pub speed: f64,
    pub ease: Option<EasingFn>,
    waypoints: Vec<Waypoint>,
    segments: Vec<Segment>,
    pub total_distance: f64,
    pub current_step: u32,
    pub max_steps: u32,
}

impl Path {
    pub fn new(path_id: &str, speed: f64, ease: Option<EasingFn>) -> Self {
        Path {
            path_id: path_id.to_string(),
            speed: if speed > 0.0 { speed } else { 1.0 },
            ease,
            waypoints: Vec::new(),
            segments: Vec::new(),
            total_distance: 0.0,
            current_step: 0,
            max_steps: 1,
        }
    }

    pub fn add_waypoint(&mut self, coord: Coord) {
        let waypoint = Waypoint::new(&self.waypoints.len().to_string(), coord);
        if let Some(prev) = self.waypoints.last() {
            let distance = find_length_of_line(prev.coord, waypoint.coord);
            self.segments.push(Segment {
                start: prev.clone(),
                end: waypoint.clone(),
                distance,
            });
            self.total_distance += distance;
        }
        self.waypoints.push(waypoint);
        self.max_steps = ((self.total_distance / self.speed).round() as u32).max(1);
    }

    pub fn is_complete(&self) -> bool {
        self.current_step >= self.max_steps
    }

    /// Advance one step, returning the coord along the path for this step.
    pub fn step(&mut self) -> Option<Coord> {
        let first = self.waypoints.first()?.coord;
        if self.segments.is_empty() {
            self.current_step = self.max_steps;
            return Some(first);
        }
        if self.current_step < self.max_steps {
            self.current_step += 1;
        }
        let mut progress = self.current_step as f64 / self.max_steps as f64;
        if let Some(ease) = self.ease {
            progress = ease(progress);
        }
        let target = self.total_distance * progress.clamp(0.0, 1.0);
        let mut travelled = 0.0;
        for segment in &self.segments {
            if travelled + segment.distance >= target {
                let local = if segment.distance == 0.0 {
                    0.0
                } else {
                    (target - travelled) / segment.distance
                };
                return Some(interpolate(segment.start.coord, segment.end.coord, local));
            }
            travelled += segment.distance;
        }
        Some(self.waypoints.last()?.coord)
    }
}

/// Per-character motion state: current coord and a set of paths.
#[derive(Clone, Debug, Default)]
pub struct Motion {
    pub current_coord: Coord,
    paths: HashMap<String, Path>,
    pub active_path: Option<String>,
}

impl Motion {
    pub fn new(coord: Coord) -> Self {
        Motion {
            current_coord: coord,
            paths: HashMap::new(),
            active_path: None,
        }
    }

    pub fn new_path(&mut self, path_id: &str, speed: f64, ease: Option<EasingFn>) -> &mut Path {
        self.paths
            .insert(path_id.to_string(), Path::new(path_id, speed, ease));
        self.paths.get_mut(path_id).expect("path just inserted")
    }

    pub fn query_path(&self, path_id: &str) -> Option<&Path> {
        self.paths.get(path_id)
    }

    pub fn query_path_mut(&mut self, path_id: &str) -> Option<&mut Path> {
        self.paths.get_mut(path_id)
    }

    pub fn activate_path(&mut self, path_id: &str) {
        if let Some(path) = self.paths.get_mut(path_id) {
            path.current_step = 0;
            self.active_path = Some(path_id.to_string());
        }
    }

    /// Step the active path one tick, updating the current coordinate.
    pub fn move_char(&mut self) {
        let Some(path_id) = self.active_path.clone() else {
            return;
        };
        if let Some(path) = self.paths.get_mut(&path_id) {
            if let Some(coord) = path.step() {
                self.current_coord = coord;
            }
            if path.is_complete() {
                self.active_path = None;
            }
        } else {
            self.active_path = None;
        }
    }

    pub fn movement_is_complete(&self) -> bool {
        self.active_path.is_none()
    }
}
