//! Motion state, ported from engine/motion.py. M0 ships coordinates only;
//! Waypoint/Segment/Path machinery lands in M1.

use crate::utils::geometry::Coord;

#[derive(Debug, Clone)]
pub struct Motion {
    pub current_coord: Coord,
    pub previous_coord: Coord,
}

impl Motion {
    pub fn new(input_coord: Coord) -> Self {
        Motion {
            current_coord: input_coord,
            previous_coord: Coord::new(-1, -1),
        }
    }

    pub fn set_coordinate(&mut self, coord: Coord) {
        self.current_coord = coord;
    }
}
