use crate::utils::easing::{linear, EasingFn};
use crate::utils::geometry::Coord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Waypoint {
    pub coord: Coord,
}

impl Waypoint {
    pub const fn new(coord: Coord) -> Self {
        Self { coord }
    }
}

#[derive(Debug, Clone)]
pub struct Path {
    waypoints: Vec<Waypoint>,
    speed: f64,
    easing: EasingFn,
    segment_index: usize,
    segment_progress: f64,
    active: bool,
    looping: bool,
}

impl Path {
    pub fn new(speed: f64) -> Self {
        Self {
            waypoints: Vec::new(),
            speed: speed.max(0.0),
            easing: linear,
            segment_index: 0,
            segment_progress: 0.0,
            active: false,
            looping: false,
        }
    }

    pub fn with_waypoints(waypoints: Vec<Waypoint>, speed: f64) -> Self {
        let mut path = Self::new(speed);
        path.waypoints = waypoints;
        path
    }

    pub fn add_waypoint(&mut self, waypoint: Waypoint) {
        self.waypoints.push(waypoint);
    }

    pub fn waypoints(&self) -> &[Waypoint] {
        &self.waypoints
    }

    pub fn set_speed(&mut self, speed: f64) {
        self.speed = speed.max(0.0);
    }

    pub fn set_easing(&mut self, easing: EasingFn) {
        self.easing = easing;
    }

    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    pub fn activate(&mut self) -> bool {
        if self.waypoints.is_empty() {
            return false;
        }

        self.segment_index = 0;
        self.segment_progress = 0.0;
        self.active = true;
        true
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn step(&mut self) -> Option<Coord> {
        if !self.active {
            return None;
        }

        if self.waypoints.len() == 1 {
            self.active = self.looping;
            return Some(self.waypoints[0].coord);
        }

        let start = self.waypoints[self.segment_index].coord;
        let end = self.waypoints[self.segment_index + 1].coord;
        let distance = start.distance(end);

        if distance <= f64::EPSILON {
            self.segment_progress = 1.0;
        } else {
            self.segment_progress += self.speed / distance;
        }

        let raw_progress = self.segment_progress.clamp(0.0, 1.0);
        let eased_progress = (self.easing)(raw_progress).clamp(0.0, 1.0);
        let position = start.lerp(end, eased_progress);

        if self.segment_progress >= 1.0 {
            self.segment_index += 1;
            self.segment_progress = 0.0;

            if self.segment_index + 1 >= self.waypoints.len() {
                if self.looping {
                    self.segment_index = 0;
                } else {
                    self.active = false;
                }
            }
        }

        Some(position)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Motion {
    active_path: Option<Path>,
}

impl Motion {
    pub fn activate_path(&mut self, mut path: Path) -> bool {
        if !path.activate() {
            return false;
        }

        self.active_path = Some(path);
        true
    }

    pub fn active_path(&self) -> Option<&Path> {
        self.active_path.as_ref()
    }

    pub fn active_path_mut(&mut self) -> Option<&mut Path> {
        self.active_path.as_mut()
    }

    pub fn deactivate(&mut self) {
        if let Some(path) = &mut self.active_path {
            path.deactivate();
        }
        self.active_path = None;
    }

    pub fn step(&mut self) -> Option<Coord> {
        let path = self.active_path.as_mut()?;
        path.step()
    }
}
