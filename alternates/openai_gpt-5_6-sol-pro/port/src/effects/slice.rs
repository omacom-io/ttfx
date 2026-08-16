
use std::collections::BTreeMap;

use super::Effect;
use crate::engine::{CharacterId, Path, Terminal, Waypoint};
use crate::utils::easing::in_out_expo;
use crate::utils::Coord;

const MOVEMENT_SPEED: f64 = 0.15;

#[derive(Debug, Clone, Copy, Default)]
pub struct Slice;

impl Slice {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Slice {
    fn name(&self) -> &str {
        "slice"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);
        let right_edge = terminal.canvas().width() as i32;

        // The upstream default is a vertical slice: each row is divided into
        // left and right halves, which enter from opposite horizontal edges.
        let mut rows: BTreeMap<i32, Vec<(i32, CharacterId)>> = BTreeMap::new();

        for character in terminal.characters() {
            rows.entry(character.position.y)
                .or_default()
                .push((character.position.x, character.id));
        }

        for row in rows.values_mut() {
            row.sort_by_key(|(x, _)| *x);
            let midpoint = row.len() / 2;

            for (index, (_, id)) in row.iter().enumerate() {
                let Some(character) = terminal.character_mut(*id) else {
                    continue;
                };

                let destination = character.position;
                let start = if index < midpoint {
                    Coord::new(-1, destination.y)
                } else {
                    Coord::new(right_edge, destination.y)
                };

                character.set_position(start);
                character.visible = true;

                let mut path = Path::with_waypoints(
                    vec![Waypoint::new(start), Waypoint::new(destination)],
                    MOVEMENT_SPEED,
                );
                path.set_easing(in_out_expo);
                character.motion.activate_path(path);
            }
        }

        let mut frames = Vec::new();

        while terminal.characters().iter().any(|character| {
            character
                .motion
                .active_path()
                .map_or(false, |path| path.is_active())
        }) {
            terminal.step();
            frames.push(terminal.render_frame());
        }

        frames
    }
}
