
use std::collections::{BTreeMap, VecDeque};

use super::Effect;
use crate::engine::{CharacterId, Path, Terminal, Waypoint};
use crate::utils::easing::in_out_quad;
use crate::utils::graphics::{Color, Style};
use crate::utils::Coord;

#[derive(Debug, Clone)]
pub struct Slide {
    movement_speed: f64,
    group_gap: usize,
}

impl Slide {
    pub fn new() -> Self {
        Self {
            movement_speed: 0.5,
            group_gap: 0,
        }
    }
}

impl Default for Slide {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Slide {
    fn name(&self) -> &str {
        "slide"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);

        if terminal.characters().is_empty() {
            return Vec::new();
        }

        let width = terminal.canvas().width() as i32;
        let height = terminal.canvas().height();

        let mut rows: BTreeMap<i32, Vec<CharacterId>> = BTreeMap::new();
        let mut destinations = BTreeMap::new();

        for character in terminal.characters() {
            rows.entry(character.position.y)
                .or_default()
                .push(character.id);
            destinations.insert(character.id, character.position);
        }

        let mut groups: VecDeque<Vec<CharacterId>> = rows.into_values().collect();

        for (row_index, group) in groups.iter().enumerate() {
            let enter_from_left = row_index % 2 == 0;

            for id in group {
                let Some(destination) = destinations.get(id).copied() else {
                    continue;
                };

                let start = if enter_from_left {
                    destination.offset(-width, 0)
                } else {
                    destination.offset(width, 0)
                };

                if let Some(character) = terminal.character_mut(*id) {
                    character.set_position(start);
                    character.visible = true;

                    let color = slide_color(destination.y, height);
                    character.set_style(Style::default().with_foreground(color));
                }
            }
        }

        let mut active = Vec::<CharacterId>::new();
        let mut gap_remaining = 0usize;
        let mut frames = Vec::new();

        while !groups.is_empty() || !active.is_empty() {
            if gap_remaining == 0 {
                if let Some(group) = groups.pop_front() {
                    for id in group {
                        let Some(destination) = destinations.get(&id).copied() else {
                            continue;
                        };

                        if let Some(character) = terminal.character_mut(id) {
                            let start = character.position;
                            let mut path = Path::with_waypoints(
                                vec![Waypoint::new(start), Waypoint::new(destination)],
                                self.movement_speed,
                            );
                            path.set_easing(in_out_quad);

                            if character.motion.activate_path(path) {
                                active.push(id);
                            } else {
                                character.set_position(destination);
                            }
                        }
                    }

                    gap_remaining = self.group_gap;
                }
            } else {
                gap_remaining -= 1;
            }

            terminal.step();

            active.retain(|id| {
                terminal
                    .character(*id)
                    .and_then(|character| character.motion.active_path())
                    .is_some_and(|path| path.is_active())
            });

            frames.push(terminal.render_frame());
        }

        frames
    }
}

fn slide_color(row: i32, height: usize) -> Color {
    const STOPS: [(u8, u8, u8); 3] = [
        (0x12, 0xc2, 0xe9),
        (0xc4, 0x71, 0xed),
        (0xf6, 0x4f, 0x59),
    ];

    let progress = if height <= 1 {
        0.0
    } else {
        (row.max(0) as f64 / (height - 1) as f64).clamp(0.0, 1.0)
    };

    let scaled = progress * (STOPS.len() - 1) as f64;
    let lower = (scaled.floor() as usize).min(STOPS.len() - 1);
    let upper = (lower + 1).min(STOPS.len() - 1);
    let local_progress = scaled - lower as f64;

    let (r1, g1, b1) = STOPS[lower];
    let (r2, g2, b2) = STOPS[upper];

    Color::rgb(
        interpolate_channel(r1, r2, local_progress),
        interpolate_channel(g1, g2, local_progress),
        interpolate_channel(b1, b2, local_progress),
    )
}

fn interpolate_channel(start: u8, end: u8, progress: f64) -> u8 {
    let value = start as f64 + (end as f64 - start as f64) * progress;
    value.round().clamp(0.0, 255.0) as u8
}
