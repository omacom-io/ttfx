use crate::utils::easing::{self, EasingFn};
use crate::utils::geometry::Coord;

#[derive(Clone, Copy, Debug)]
pub struct Waypoint {
    pub position: Coord,
    pub easing: EasingFn,
}

impl Waypoint {
    pub fn new(position: Coord) -> Self {
        Self {
            position,
            easing: easing::linear,
        }
    }

    pub fn with_easing(position: Coord, easing: EasingFn) -> Self {
        Self { position, easing }
    }
}

#[derive(Clone, Debug)]
pub struct Segment {
    pub start: Waypoint,
    pub end: Waypoint,
}

impl Segment {
    pub fn new(start: Waypoint, end: Waypoint) -> Self {
        Self { start, end }
    }

    pub fn length(&self) -> f32 {
        self.start.position.distance(self.end.position)
    }

    pub fn point_at(&self, t: f32) -> Coord {
        let eased_t = (self.start.easing)(t);
        self.start.position.lerp(self.end.position, eased_t)
    }
}

#[derive(Clone, Debug)]
pub struct Path {
    pub segments: Vec<Segment>,
}

impl Path {
    pub fn new() -> Self {
        Self { segments: Vec::new() }
    }

    pub fn add_segment(&mut self, segment: Segment) {
        self.segments.push(segment);
    }

    pub fn length(&self) -> f32 {
        self.segments.iter().map(|s| s.length()).sum()
    }
}

#[derive(Clone, Debug)]
pub struct Motion {
    pub paths: Vec<Path>,
}

impl Motion {
    pub fn new() -> Self {
        Self { paths: Vec::new() }
    }

    pub fn add_path(&mut self, path: Path) {
        self.paths.push(path);
    }
}
