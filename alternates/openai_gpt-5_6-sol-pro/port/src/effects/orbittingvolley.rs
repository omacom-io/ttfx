
use std::collections::VecDeque;

use super::Effect;
use crate::engine::{CharacterId, Path, Terminal, Waypoint};
use crate::utils::easing::out_sine;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Style};

#[derive(Debug, Clone)]
pub struct Orbittingvolley {
    launcher_symbol: char,
    launcher_color: Color,
    movement_speed: f64,
    launcher_movement_speed: f64,
    volley_size: usize,
    launch_delay: usize,
}

impl Orbittingvolley {
    pub fn new() -> Self {
        Self {
            launcher_symbol: '█',
            launcher_color: Color::rgb(255, 255, 255),
            movement_speed: 1.0,
            launcher_movement_speed: 0.5,
            volley_size: 4,
            launch_delay: 2,
        }
    }
}

impl Default for Orbittingvolley {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Orbittingvolley {
    fn name(&self) -> &str {
        "orbittingvolley"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);
        let max_x = terminal.canvas().width().saturating_sub(1) as i32;
        let max_y = terminal.canvas().height().saturating_sub(1) as i32;

        let corners = [
            Coord::new(0, 0),
            Coord::new(max_x, 0),
            Coord::new(max_x, max_y),
            Coord::new(0, max_y),
        ];

        let original_characters: Vec<(CharacterId, Coord)> = terminal
            .characters()
            .iter()
            .map(|character| (character.id, character.position))
            .collect();

        for character in terminal.characters_mut() {
            character.visible = false;
        }

        let launcher_ids: [CharacterId; 4] = std::array::from_fn(|index| {
            let id = terminal.add_character(self.launcher_symbol, corners[index]);

            if let Some(launcher) = terminal.character_mut(id) {
                launcher.set_appearance(
                    self.launcher_symbol,
                    Style::default().with_foreground(self.launcher_color),
                );
                launcher.visible = true;
            }

            id
        });

        let mut magazines: [VecDeque<CharacterId>; 4] =
            std::array::from_fn(|_| VecDeque::new());

        for &(id, destination) in &original_characters {
            let launcher_index = nearest_corner(destination, &corners);
            magazines[launcher_index].push_back(id);
        }

        for magazine in &mut magazines {
            let slice = magazine.make_contiguous();
            slice.sort_by_key(|id| {
                terminal
                    .character(*id)
                    .map(|character| {
                        let center_distance = distance_squared_from_center(
                            character.position,
                            max_x,
                            max_y,
                        );
                        (center_distance, character.id)
                    })
                    .unwrap_or((0, *id))
            });
        }

        let mut frames = Vec::new();
        let mut launcher_progress = 0.0;
        let mut launch_delay_remaining = 0usize;

        loop {
            if launch_delay_remaining == 0 {
                for launcher_index in 0..launcher_ids.len() {
                    let launcher_position = terminal
                        .character(launcher_ids[launcher_index])
                        .map(|launcher| launcher.position)
                        .unwrap_or(corners[launcher_index]);

                    for _ in 0..self.volley_size {
                        let Some(character_id) = magazines[launcher_index].pop_front() else {
                            break;
                        };

                        let Some(destination) = original_characters
                            .iter()
                            .find_map(|(id, coord)| (*id == character_id).then_some(*coord))
                        else {
                            continue;
                        };

                        let style = final_style(destination, max_x, max_y);
                        let mut path = Path::with_waypoints(
                            vec![
                                Waypoint::new(launcher_position),
                                Waypoint::new(destination),
                            ],
                            self.movement_speed,
                        );
                        path.set_easing(out_sine);

                        if let Some(character) = terminal.character_mut(character_id) {
                            character.set_position(launcher_position);
                            character.set_appearance(character.input_symbol, style);
                            character.visible = true;
                            character.motion.activate_path(path);
                        }
                    }
                }

                launch_delay_remaining = self.launch_delay;
            } else {
                launch_delay_remaining -= 1;
            }

            terminal.step();

            let orbit_span = f64::from(max_x.max(1));
            launcher_progress += self.launcher_movement_speed / orbit_span;
            if launcher_progress > 1.0 {
                launcher_progress -= 1.0;
            }

            update_launchers(
                &mut terminal,
                launcher_ids,
                launcher_progress,
                max_x,
                max_y,
            );

            frames.push(terminal.render_frame());

            let magazines_empty = magazines.iter().all(VecDeque::is_empty);
            let characters_at_rest = original_characters.iter().all(|(id, _)| {
                terminal
                    .character(*id)
                    .and_then(|character| character.motion.active_path())
                    .is_none_or(|path| !path.is_active())
            });

            if magazines_empty && characters_at_rest {
                for launcher_id in launcher_ids {
                    if let Some(launcher) = terminal.character_mut(launcher_id) {
                        launcher.visible = false;
                    }
                }

                for &(id, destination) in &original_characters {
                    if let Some(character) = terminal.character_mut(id) {
                        character.set_position(destination);
                        character.visible = true;
                    }
                }

                let final_frame = terminal.render_frame();
                if frames.last() != Some(&final_frame) {
                    frames.push(final_frame);
                }
                break;
            }
        }

        if frames.is_empty() {
            frames.push(terminal.render_frame());
        }

        frames
    }
}

fn nearest_corner(coord: Coord, corners: &[Coord; 4]) -> usize {
    corners
        .iter()
        .enumerate()
        .min_by_key(|(index, corner)| (coord.manhattan_distance(**corner), *index))
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn distance_squared_from_center(coord: Coord, max_x: i32, max_y: i32) -> i64 {
    let doubled_x = i64::from(coord.x) * 2 - i64::from(max_x);
    let doubled_y = i64::from(coord.y) * 2 - i64::from(max_y);
    doubled_x * doubled_x + doubled_y * doubled_y
}

fn final_style(coord: Coord, max_x: i32, max_y: i32) -> Style {
    const START: (u8, u8, u8) = (49, 233, 129);
    const END: (u8, u8, u8) = (27, 231, 255);

    let denominator = (max_x + max_y).max(1) as f64;
    let progress = f64::from(coord.x + coord.y).clamp(0.0, denominator) / denominator;

    let interpolate = |start: u8, end: u8| {
        (f64::from(start) + (f64::from(end) - f64::from(start)) * progress)
            .round()
            .clamp(0.0, 255.0) as u8
    };

    Style::default().with_foreground(Color::rgb(
        interpolate(START.0, END.0),
        interpolate(START.1, END.1),
        interpolate(START.2, END.2),
    ))
}

fn update_launchers(
    terminal: &mut Terminal,
    launcher_ids: [CharacterId; 4],
    progress: f64,
    max_x: i32,
    max_y: i32,
) {
    let progress = progress.clamp(0.0, 1.0);
    let x_forward = (f64::from(max_x) * progress).trunc() as i32;
    let y_forward = (f64::from(max_y) * progress).trunc() as i32;

    let positions = [
        Coord::new(x_forward, 0),
        Coord::new(max_x, y_forward),
        Coord::new(max_x - x_forward, max_y),
        Coord::new(0, max_y - y_forward),
    ];

    for (launcher_id, position) in launcher_ids.into_iter().zip(positions) {
        if let Some(launcher) = terminal.character_mut(launcher_id) {
            launcher.set_position(position);
        }
    }
}
