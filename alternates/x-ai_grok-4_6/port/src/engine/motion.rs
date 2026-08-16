use std::collections::HashMap;
use std::fmt;

use crate::utils::easing::Ease;
use crate::utils::geometry::{
    find_coord_on_bezier_curve, find_length_of_bezier_curve, find_length_of_line, lerp_coord, Coord,
};
use crate::utils::round_half_even;

#[derive(Clone, Debug)]
pub struct Waypoint {
    pub waypoint_id: String,
    pub coord: Coord,
    pub bezier_control: Option<Coord>,
}

#[derive(Clone, Debug)]
pub struct Segment {
    pub start: Waypoint,
    pub end: Waypoint,
    pub control: Option<Coord>,
    pub length: f64,
}

impl Segment {
    pub fn point_at(&self, t: f64) -> Coord {
        if let Some(control) = self.control {
            find_coord_on_bezier_curve(self.start.coord, control, self.end.coord, t)
        } else {
            lerp_coord(self.start.coord, self.end.coord, t)
        }
    }
}

#[derive(Clone, Debug)]
pub struct Path {
    pub path_id: String,
    pub speed: f64,
    pub ease: Option<Ease>,
    pub loop_path: bool,
    pub complete: bool,
    waypoints: Vec<Waypoint>,
    segments: Vec<Segment>,
    origin: Option<Coord>,
    current_step: usize,
    max_steps: usize,
    total_length: f64,
}

impl Path {
    pub fn new(path_id: impl Into<String>, speed: f64) -> Self {
        Self {
            path_id: path_id.into(),
            speed: if speed <= 0.0 { 1.0 } else { speed },
            ease: None,
            loop_path: false,
            complete: false,
            waypoints: Vec::new(),
            segments: Vec::new(),
            origin: None,
            current_step: 0,
            max_steps: 1,
            total_length: 0.0,
        }
    }

    pub fn new_waypoint(
        &mut self,
        waypoint_id: impl Into<String>,
        coord: Coord,
        bezier_control: Option<Coord>,
    ) -> &Waypoint {
        self.waypoints.push(Waypoint {
            waypoint_id: waypoint_id.into(),
            coord,
            bezier_control,
        });
        self.waypoints.last().expect("waypoint just pushed")
    }

    pub fn query_waypoint(&self, waypoint_id: &str) -> Option<&Waypoint> {
        self.waypoints
            .iter()
            .find(|wp| wp.waypoint_id == waypoint_id)
    }

    pub fn waypoints(&self) -> &[Waypoint] {
        &self.waypoints
    }

    pub fn reset(&mut self) {
        self.current_step = 0;
        self.complete = false;
    }

    fn rebuild_segments(&mut self) {
        self.segments.clear();
        let mut points: Vec<Waypoint> = Vec::new();
        if let Some(origin) = self.origin {
            points.push(Waypoint {
                waypoint_id: "_origin".into(),
                coord: origin,
                bezier_control: None,
            });
        }
        points.extend(self.waypoints.iter().cloned());
        for pair in points.windows(2) {
            let start = pair[0].clone();
            let end = pair[1].clone();
            let control = end.bezier_control;
            let length = if let Some(c) = control {
                find_length_of_bezier_curve(start.coord, c, end.coord)
            } else {
                find_length_of_line(start.coord, end.coord)
            };
            self.segments.push(Segment {
                start,
                end,
                control,
                length,
            });
        }
        self.total_length = self.segments.iter().map(|s| s.length).sum();
        self.max_steps = if self.speed <= 0.0 {
            1
        } else {
            round_half_even(self.total_length / self.speed).max(1) as usize
        };
        self.current_step = 0;
        self.complete = self.segments.is_empty();
    }

    pub fn step(&mut self) -> Option<Coord> {
        if self.segments.is_empty() {
            self.complete = true;
            return self.end_coord();
        }
        if self.complete {
            if self.loop_path {
                self.current_step = 0;
                self.complete = false;
            } else {
                return self.end_coord();
            }
        }
        self.current_step = self.current_step.saturating_add(1);
        if self.current_step >= self.max_steps {
            self.complete = !self.loop_path;
            if self.loop_path {
                self.current_step = 0;
            }
            return self.end_coord();
        }
        let mut t = self.current_step as f64 / self.max_steps as f64;
        if let Some(ease) = self.ease {
            t = ease.apply(t.clamp(0.0, 1.0));
        }
        Some(self.coord_at_progress(t.clamp(0.0, 1.0)))
    }

    fn coord_at_progress(&self, t: f64) -> Coord {
        if self.segments.is_empty() {
            return self.end_coord().unwrap_or_default();
        }
        let target = t * self.total_length;
        let last = self.segments.len() - 1;
        let mut acc = 0.0;
        for (i, seg) in self.segments.iter().enumerate() {
            if acc + seg.length >= target || i == last {
                let local = if seg.length <= f64::EPSILON {
                    1.0
                } else {
                    ((target - acc) / seg.length).clamp(0.0, 1.0)
                };
                return seg.point_at(local);
            }
            acc += seg.length;
        }
        self.end_coord().unwrap_or_default()
    }

    fn end_coord(&self) -> Option<Coord> {
        self.waypoints
            .last()
            .map(|wp| wp.coord)
            .or(self.origin)
    }
}

#[derive(Clone, Debug)]
pub struct Motion {
    pub current_coord: Coord,
    pub input_coord: Coord,
    pub active_path: Option<String>,
    paths: HashMap<String, Path>,
    path_ids: Vec<String>,
}

impl Motion {
    pub fn new(coord: Coord) -> Self {
        Self {
            current_coord: coord,
            input_coord: coord,
            active_path: None,
            paths: HashMap::new(),
            path_ids: Vec::new(),
        }
    }

    pub fn new_path(&mut self, path_id: impl Into<String>, speed: f64) -> &mut Path {
        let path_id = path_id.into();
        if !self.paths.contains_key(&path_id) {
            self.path_ids.push(path_id.clone());
            self.paths
                .insert(path_id.clone(), Path::new(path_id.clone(), speed));
        }
        self.paths.get_mut(&path_id).expect("path just inserted")
    }

    pub fn query_path(&self, path_id: &str) -> Result<&Path, MotionError> {
        self.paths
            .get(path_id)
            .ok_or_else(|| MotionError::PathNotFound(path_id.to_string()))
    }

    pub fn activate_path(&mut self, path_id: &str) {
        if let Some(path) = self.paths.get_mut(path_id) {
            path.origin = Some(self.current_coord);
            path.reset();
            path.rebuild_segments();
            self.active_path = Some(path_id.to_string());
        }
    }

    pub fn set_coordinate(&mut self, coord: Coord) {
        self.current_coord = coord;
    }

    pub fn move_character(&mut self) -> bool {
        let Some(path_id) = self.active_path.clone() else {
            return false;
        };
        let Some(path) = self.paths.get_mut(&path_id) else {
            return false;
        };
        if path.complete && !path.loop_path {
            self.active_path = None;
            return false;
        }
        if let Some(coord) = path.step() {
            self.current_coord = coord;
        }
        if path.complete && !path.loop_path {
            self.active_path = None;
            return false;
        }
        true
    }

    pub fn is_active(&self) -> bool {
        self.active_path.is_some()
    }

    pub fn path_ids(&self) -> &[String] {
        &self.path_ids
    }
}

#[derive(Clone, Debug)]
pub enum MotionError {
    DuplicatePathId(String),
    PathNotFound(String),
    DuplicateWaypointId(String),
    WaypointNotFound(String),
    ActivateEmptyPath(String),
    InvalidSpeed(f64),
}

impl fmt::Display for MotionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePathId(id) => write!(f, "duplicate path id '{id}'"),
            Self::PathNotFound(id) => write!(f, "path '{id}' not found"),
            Self::DuplicateWaypointId(id) => write!(f, "duplicate waypoint id '{id}'"),
            Self::WaypointNotFound(id) => write!(f, "waypoint '{id}' not found"),
            Self::ActivateEmptyPath(id) => write!(f, "cannot activate empty path '{id}'"),
            Self::InvalidSpeed(s) => write!(f, "invalid path speed {s}"),
        }
    }
}

impl std::error::Error for MotionError {}
