use std::collections::HashMap;

use super::Effect;
use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::character::CharacterId;
use crate::engine::motion::Path;
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::{self, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Port of `effect_middleout.py`: characters first collapse onto the
/// horizontal center line (preserving their original column), then expand
/// back out to their original position, giving a converge-then-expand
/// "middle out" reveal.
pub struct Middleout {
    speed: f64,
}

impl Middleout {
    pub fn new() -> Self {
        Middleout { speed: 1.0 }
    }
}

impl Effect for Middleout {
    fn name(&self) -> &str {
        "middleout"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;
        let center_row = height / 2;
        let center_column = width / 2;
        let center_point = Coord::new(center_column, center_row);
        let max_dist = geometry::distance(Coord::new(0, 0), Coord::new(width, height)).max(1.0);

        // Gradient used to color characters by distance from the center,
        // standing in for the Python effect's fg gradient mapping.
        let gradient = Gradient::new(&[Color::Rgb(60, 220, 255), Color::Rgb(255, 80, 220)], 40);

        let mut character_steps: HashMap<CharacterId, (u32, u32)> = HashMap::new();

        for character in terminal.get_characters_mut() {
            let final_coord = character.input_coord;
            // Collapse point: preserve column, snap row to the canvas
            // center (mirrors `expand_direction == "vertical"` branch).
            let center_coord = Coord::new(final_coord.column, center_row);

            let mut center_path = Path::new("center", self.speed);
            center_path.ease = Some(easing::ease_in_out_sine);
            center_path.add_waypoint(final_coord);
            center_path.add_waypoint(center_coord);
            let center_distance = center_path.total_distance();
            let center_steps = (center_distance / self.speed).round().max(0.0) as u32;

            let mut full_path = Path::new("full", self.speed);
            full_path.ease = Some(easing::ease_out_quad);
            full_path.add_waypoint(center_coord);
            full_path.add_waypoint(final_coord);
            let full_distance = full_path.total_distance();
            let full_steps = (full_distance / self.speed).round().max(0.0) as u32;

            character.motion.add_path(center_path);
            character.motion.add_path(full_path);
            character.motion.activate_path("center");

            character_steps.insert(character.id, (center_steps, full_steps));

            let distance_from_center = geometry::distance(final_coord, center_point);
            let normalized = (distance_from_center / max_dist).clamp(0.0, 1.0);
            let idx = (normalized * (gradient.len().saturating_sub(1)) as f64).round() as usize;
            let color = gradient.get(idx).unwrap_or(Color::Rgb(255, 255, 255));

            let mut visual = CharacterVisual::new(character.input_symbol);
            visual.colors = Some(ColorPair::new(Some(color), None));
            visual.formatted_symbol = visual.format_symbol();

            let mut scene = Scene::new("colored");
            scene.add_frame(visual, 1);
            character.animation.add_scene(scene);
            character.animation.activate_scene("colored");
        }

        let max_steps = character_steps
            .values()
            .map(|(c, f)| c + f)
            .max()
            .unwrap_or(0);
        let hold_frames: u32 = 30;
        let total_frames = max_steps + hold_frames;

        let mut frames = Vec::with_capacity(total_frames as usize + 1);
        for step in 0..total_frames {
            for character in terminal.get_characters_mut() {
                if let Some(&(center_steps, _full_steps)) = character_steps.get(&character.id) {
                    if step == center_steps {
                        character.motion.activate_path("full");
                    }
                }
                character.motion.step();
                character.animation.step_animation();
            }
            frames.push(terminal.render());
        }

        frames
    }
}
