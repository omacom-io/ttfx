//! Pour effect (port of terminaltexteffects/effects/effect_pour.py).
//!
//! Characters are grouped by row from the top of the canvas to the bottom.
//! Each group is poured into place: characters start at the top edge of the
//! canvas and fall to their input coordinate, released a few at a time with a
//! configurable gap between releases. Rows alternate pour direction
//! (left-to-right, then right-to-left) like liquid filling a vessel. While a
//! character falls it animates through a gradient from the starting color to
//! its final color, which is drawn from a vertical gradient across the canvas.

use std::collections::VecDeque;

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

pub struct Pour {
    /// Number of characters released per pour tick.
    pour_speed: usize,
    /// Speed of the falling characters along their path.
    movement_speed: f64,
    /// Frames to wait between character releases.
    gap: u32,
    /// Color characters start with while pouring.
    starting_color: Color,
    /// Stops for the final gradient applied across the canvas (vertical).
    final_gradient_stops: Vec<Color>,
    /// Interpolation steps between final gradient stops.
    final_gradient_steps: usize,
    /// Steps used for each character's pour (starting -> final) gradient.
    pour_gradient_steps: usize,
    /// Frame duration for each pour gradient step.
    pour_frame_duration: u32,
}

impl Pour {
    pub fn new() -> Self {
        Pour {
            pour_speed: 1,
            movement_speed: 0.2,
            gap: 1,
            starting_color: Color::new(0xFF, 0xFF, 0xFF),
            final_gradient_stops: vec![
                Color::new(0x8A, 0x00, 0x8A),
                Color::new(0x00, 0xD1, 0xFF),
                Color::new(0xFF, 0xFF, 0xFF),
            ],
            final_gradient_steps: 12,
            pour_gradient_steps: 10,
            pour_frame_duration: 5,
        }
    }
}

impl Default for Pour {
    fn default() -> Self {
        Pour::new()
    }
}

impl Effect for Pour {
    fn name(&self) -> &str {
        "pour"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let height = terminal.canvas.height as i32;
        let top_row = height;

        let final_gradient = Gradient::new(&self.final_gradient_stops, self.final_gradient_steps);

        // Group characters by row, top-to-bottom, columns ascending; alternate
        // the pour direction on every other row.
        let mut groups: Vec<Vec<usize>> = Vec::new();
        {
            let characters = terminal.get_characters();
            for (group_index, row) in (1..=height).rev().enumerate() {
                let mut members: Vec<(i32, usize)> = characters
                    .iter()
                    .filter(|c| c.input_coord.row == row)
                    .map(|c| (c.input_coord.column, c.character_id))
                    .collect();
                members.sort_by_key(|(column, _)| *column);
                let mut ids: Vec<usize> = members.into_iter().map(|(_, id)| id).collect();
                if group_index % 2 == 1 {
                    ids.reverse();
                }
                if !ids.is_empty() {
                    groups.push(ids);
                }
            }
        }

        // Prepare each character: hidden, positioned at the top of its column,
        // with a path to its input coordinate and a pour gradient scene.
        for character in terminal.get_characters_mut() {
            character.is_visible = false;
            let input_coord = character.input_coord;
            let symbol = character.input_symbol;

            let fraction = if height > 1 {
                (height - input_coord.row) as f64 / (height - 1) as f64
            } else {
                0.0
            };
            let final_color = final_gradient
                .get_color_at_fraction(fraction)
                .unwrap_or(self.starting_color);

            let start_coord = Coord::new(input_coord.column, top_row);
            character.motion.current_coord = start_coord;

            let path = character
                .motion
                .new_path("input_coord", self.movement_speed, Some(easing::in_quad));
            path.add_waypoint(start_coord);
            path.add_waypoint(input_coord);

            let pour_gradient =
                Gradient::new(&[self.starting_color, final_color], self.pour_gradient_steps);
            let scene = character.animation.new_scene("pour", false);
            let last = pour_gradient.spectrum.len().saturating_sub(1);
            for (idx, color) in pour_gradient.spectrum.iter().enumerate() {
                let duration = if idx == last {
                    1
                } else {
                    self.pour_frame_duration
                };
                scene.add_frame(symbol, duration, ColorPair::fg(*color), false);
            }
        }

        // Run the effect: release characters group by group with a gap between
        // releases, ticking the terminal and capturing a frame each iteration.
        let mut frames: Vec<String> = Vec::new();
        let mut pending_groups: VecDeque<Vec<usize>> = groups.into();
        let mut current_group: VecDeque<usize> = VecDeque::new();
        let mut gap_timer: u32 = 0;
        let max_frames = 20_000usize;

        loop {
            if current_group.is_empty() {
                if let Some(group) = pending_groups.pop_front() {
                    current_group = group.into();
                }
            }

            if !current_group.is_empty() {
                if gap_timer == 0 {
                    for _ in 0..self.pour_speed.max(1) {
                        let Some(id) = current_group.pop_front() else {
                            break;
                        };
                        terminal.set_character_visibility(id, true);
                        if let Some(character) = terminal
                            .get_characters_mut()
                            .iter_mut()
                            .find(|c| c.character_id == id)
                        {
                            character.motion.activate_path("input_coord");
                            character.animation.activate_scene("pour");
                        }
                    }
                    gap_timer = self.gap;
                } else {
                    gap_timer -= 1;
                }
            }

            terminal.tick();
            frames.push(terminal.get_formatted_output_string());

            let any_active = terminal.get_characters().iter().any(|c| c.is_active());
            if current_group.is_empty() && pending_groups.is_empty() && !any_active {
                break;
            }
            if frames.len() >= max_frames {
                break;
            }
        }

        frames
    }
}
