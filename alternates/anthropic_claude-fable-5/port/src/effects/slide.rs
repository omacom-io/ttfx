//! Slide effect: characters slide into position from outside the canvas,
//! grouped by row and released with a gap between groups, while fading
//! from the first gradient stop to their final gradient color.
//!
//! Port of terminaltexteffects/effects/effect_slide.py (default config:
//! grouping="row", movement_speed=0.5, gap=10, merge=false,
//! reverse_direction=false, movement_easing=in_out_quad,
//! final_gradient_stops=(833ab4, fd1d1d, fcb045), final_gradient_steps=12,
//! final_gradient_frames=10, vertical gradient direction).

use std::collections::VecDeque;

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

const MOVEMENT_SPEED: f64 = 0.5;
const GAP: u32 = 10;
const FINAL_GRADIENT_STEPS: usize = 12;
const FINAL_GRADIENT_FRAMES: u32 = 10;
const CHAR_GRADIENT_STEPS: usize = 10;
const MERGE: bool = false;
const REVERSE_DIRECTION: bool = false;
const MAX_FRAMES: usize = 20_000;

pub struct Slide;

impl Slide {
    pub fn new() -> Self {
        Slide
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
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let height = terminal.canvas.height as i32;
        let width = terminal.canvas.width as i32;

        let stops = [
            Color::from_hex("833ab4").expect("valid hex"),
            Color::from_hex("fd1d1d").expect("valid hex"),
            Color::from_hex("fcb045").expect("valid hex"),
        ];
        // Vertical gradient across the canvas determines each character's
        // final color (row 1 at the bottom maps to fraction 0).
        let final_gradient = Gradient::new(&stops, FINAL_GRADIENT_STEPS);

        // Group characters by row, top to bottom, left to right within a row
        // (mirrors CharacterGroup.ROW_TOP_TO_BOTTOM).
        let mut groups: Vec<Vec<usize>> = Vec::new();
        for row in (1..=height).rev() {
            let mut group: Vec<usize> = terminal
                .get_characters()
                .iter()
                .enumerate()
                .filter(|(_, c)| c.input_coord.row == row)
                .map(|(i, _)| i)
                .collect();
            group.sort_by_key(|&i| terminal.get_characters()[i].input_coord.column);
            if !group.is_empty() {
                groups.push(group);
            }
        }

        // Build: place each character off-canvas, give it a path back to its
        // input coordinate, and prepare its gradient fade scene.
        for (group_index, group) in groups.iter().enumerate() {
            // Rows slide in from the left by default; from the right when
            // reversed; alternating sides when merging.
            let from_right = if MERGE {
                group_index % 2 == 0
            } else {
                REVERSE_DIRECTION
            };
            let starting_column = if from_right { width + 1 } else { 0 };

            for &idx in group {
                let (symbol, input_coord) = {
                    let character = &terminal.get_characters()[idx];
                    (character.input_symbol, character.input_coord)
                };
                let start_coord = Coord::new(starting_column, input_coord.row);

                let fraction = if height > 1 {
                    (input_coord.row - 1) as f64 / (height - 1) as f64
                } else {
                    0.0
                };
                let final_color = final_gradient
                    .get_color_at_fraction(fraction)
                    .unwrap_or(stops[0]);
                let char_gradient =
                    Gradient::new(&[stops[0], final_color], CHAR_GRADIENT_STEPS);

                let character = &mut terminal.get_characters_mut()[idx];
                character.motion.current_coord = start_coord;

                let path = character.motion.new_path(
                    "input_coord",
                    MOVEMENT_SPEED,
                    Some(easing::in_out_quad),
                );
                path.add_waypoint(start_coord);
                path.add_waypoint(input_coord);

                let scene = character.animation.new_scene("gradient", false);
                for color in &char_gradient.spectrum {
                    scene.add_frame(symbol, FINAL_GRADIENT_FRAMES, ColorPair::fg(*color), false);
                }
            }
        }

        // Run: release one group every GAP frames, then tick until everything
        // has finished moving and fading.
        let mut pending: VecDeque<Vec<usize>> = groups.into();
        let mut frames: Vec<String> = Vec::new();
        let mut gap_timer: u32 = 0;

        loop {
            if gap_timer == 0 {
                if let Some(group) = pending.pop_front() {
                    for idx in group {
                        let character = &mut terminal.get_characters_mut()[idx];
                        character.is_visible = true;
                        character.motion.activate_path("input_coord");
                        character.animation.activate_scene("gradient");
                    }
                    gap_timer = GAP;
                }
            } else {
                gap_timer -= 1;
            }

            let active = terminal.tick();
            frames.push(terminal.get_formatted_output_string());

            if pending.is_empty() && active == 0 {
                break;
            }
            if frames.len() >= MAX_FRAMES {
                break;
            }
        }

        frames
    }
}
