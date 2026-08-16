//! Spray paint effect: characters fly in from a compass-direction origin
//! point on the canvas, staggered in small batches (mirrors the shape of
//! `terminaltexteffects/effects/effect_spray.py`), settling into their
//! input position and swapping from the spray color to a final gradient
//! color once arrived.

use super::Effect;
use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::motion::Path;
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Compass-direction (plus center) origin point the spray emanates from,
/// mirroring upstream's `spray_position` choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SprayPosition {
    N,
    Ne,
    E,
    Se,
    S,
    Sw,
    W,
    Nw,
    Center,
}

pub struct Spray {
    spray_position: SprayPosition,
    spray_volume: f64,
    movement_speed: (f64, f64),
    spray_color: Color,
    final_gradient_stops: Vec<Color>,
    final_gradient_steps: usize,
}

impl Spray {
    pub fn new() -> Self {
        Spray {
            spray_position: SprayPosition::Center,
            spray_volume: 0.005,
            movement_speed: (0.4, 1.0),
            spray_color: Color::Rgb(0x00, 0xff, 0xff),
            final_gradient_stops: vec![Color::Rgb(0x8A, 0x00, 0x8A), Color::Rgb(0x00, 0xd5, 0xff)],
            final_gradient_steps: 12,
        }
    }

    fn origin(&self, width: usize, height: usize) -> Coord {
        let right = width.saturating_sub(1) as i32;
        let bottom = height.saturating_sub(1) as i32;
        let center_col = right / 2;
        let center_row = bottom / 2;
        match self.spray_position {
            SprayPosition::N => Coord::new(center_col, 0),
            SprayPosition::Ne => Coord::new(right, 0),
            SprayPosition::E => Coord::new(right, center_row),
            SprayPosition::Se => Coord::new(right, bottom),
            SprayPosition::S => Coord::new(center_col, bottom),
            SprayPosition::Sw => Coord::new(0, bottom),
            SprayPosition::W => Coord::new(0, center_row),
            SprayPosition::Nw => Coord::new(0, 0),
            SprayPosition::Center => Coord::new(center_col, center_row),
        }
    }
}

/// Tiny deterministic xorshift32 PRNG so per-character jitter, launch speed
/// and launch order are stable without depending on an external RNG module
/// (not part of this crate's file list).
fn next_rand(state: &mut u32) -> f64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    (x as f64) / (u32::MAX as f64)
}

impl Effect for Spray {
    fn name(&self) -> &str {
        "spray"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let (width, height) = (terminal.config.width, terminal.config.height);
        let origin = self.origin(width, height);

        let final_gradient = Gradient::new(&self.final_gradient_stops, self.final_gradient_steps);

        let ids: Vec<u32> = terminal.get_characters().iter().map(|c| c.id).collect();
        let total_chars = ids.len();

        let mut seed: u32 = 0x9E37_79B9;
        let mut order: Vec<u32> = ids.clone();
        for i in (1..order.len()).rev() {
            let r = next_rand(&mut seed);
            let j = (r * (i as f64 + 1.0)) as usize % (i + 1);
            order.swap(i, j);
        }

        for &id in &ids {
            let character = terminal.get_character_mut(id).unwrap();
            character.visible = false;

            let jitter_x = (next_rand(&mut seed) - 0.5) * 2.0;
            let jitter_y = (next_rand(&mut seed) - 0.5) * 2.0;
            let start_col = (origin.column as f64 + jitter_x).round() as i32;
            let start_row = (origin.row as f64 + jitter_y).round() as i32;
            let start_coord = Coord::new(
                start_col.clamp(0, width.saturating_sub(1) as i32),
                start_row.clamp(0, height.saturating_sub(1) as i32),
            );
            character.motion.current_pos = (start_coord.column as f64, start_coord.row as f64);
            character.motion.current_coord = start_coord;

            let speed_frac = next_rand(&mut seed);
            let speed = self.movement_speed.0 + speed_frac * (self.movement_speed.1 - self.movement_speed.0);

            let mut path = Path::new("spray", speed);
            path.ease = Some(easing::ease_out_expo);
            path.add_waypoint(start_coord);
            path.add_waypoint(character.input_coord);
            character.motion.add_path(path);

            let mut spray_visual = CharacterVisual::new(character.input_symbol);
            spray_visual.colors = Some(ColorPair::new(Some(self.spray_color), None));
            spray_visual.formatted_symbol = spray_visual.format_symbol();
            let mut spray_scene = Scene::new("spray");
            spray_scene.is_looping = true;
            spray_scene.add_frame(spray_visual, 1);
            character.animation.add_scene(spray_scene);

            let final_color = if final_gradient.is_empty() {
                self.spray_color
            } else {
                let final_index = (character.input_coord.column.max(0) as usize) % final_gradient.len();
                final_gradient.get(final_index).unwrap_or(self.spray_color)
            };
            let mut final_visual = CharacterVisual::new(character.input_symbol);
            final_visual.colors = Some(ColorPair::new(Some(final_color), None));
            final_visual.formatted_symbol = final_visual.format_symbol();
            let mut final_scene = Scene::new("final");
            final_scene.is_looping = true;
            final_scene.add_frame(final_visual, 1);
            character.animation.add_scene(final_scene);

            character.animation.activate_scene("spray");
        }

        // Stagger launches: spray_volume fraction of the not-yet-launched
        // queue is released each tick, mirroring the paint-can spray effect.
        let mut pending: Vec<u32> = order;
        let mut launched: Vec<u32> = Vec::with_capacity(total_chars);
        let mut frames_out: Vec<String> = Vec::new();

        loop {
            if !pending.is_empty() {
                let launch_count = ((pending.len() as f64) * self.spray_volume).ceil().max(1.0) as usize;
                let launch_count = launch_count.min(pending.len());
                for _ in 0..launch_count {
                    let id = pending.remove(0);
                    if let Some(character) = terminal.get_character_mut(id) {
                        character.visible = true;
                        character.motion.activate_path("spray");
                    }
                    launched.push(id);
                }
            }

            terminal.step_animation();

            // Characters whose path has fully arrived switch to the final
            // scene (stand-in for upstream's PATH_COMPLETE event dispatch).
            for &id in &launched {
                if let Some(character) = terminal.get_character_mut(id) {
                    if character.motion.current_coord == character.input_coord
                        && character.animation.active_scene_id.as_deref() != Some("final")
                    {
                        character.animation.activate_scene("final");
                    }
                }
            }

            frames_out.push(terminal.render());

            let all_settled = pending.is_empty()
                && launched.iter().all(|&id| {
                    terminal
                        .get_character(id)
                        .map(|c| c.motion.current_coord == c.input_coord)
                        .unwrap_or(true)
                });

            if all_settled && frames_out.len() > 1 {
                break;
            }
            if frames_out.len() > 10_000 {
                break; // safety valve against runaway loops
            }
        }

        frames_out
    }
}
