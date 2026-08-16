
use super::Effect;
use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::motion::Path;
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair};

/// A simplified port of `effect_blackhole.py`: every character is pulled
/// from its starting position into a "singularity" at the canvas center,
/// pauses there tinted as part of the hole, then is flung back out along
/// the reverse path to its original position, matching the collapse /
/// release shape of the upstream effect within the primitives available
/// here (no event handler / circle-sampling helpers in this skeleton).
pub struct Blackhole {}

impl Blackhole {
    pub fn new() -> Self {
        Blackhole {}
    }
}

impl Effect for Blackhole {
    fn name(&self) -> &str {
        "blackhole"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);

        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;
        let center = Coord::new((width - 1) / 2, (height - 1) / 2);

        let hole_color = Color::Rgb(90, 0, 130);

        let char_ids: Vec<u32> = terminal.get_characters().iter().map(|c| c.id).collect();

        // Build the singularity (inward) and explode (outward) paths plus
        // the "hole" tint scene for every character.
        for id in &char_ids {
            let (input_coord, symbol) = {
                let character = terminal.get_character(*id).unwrap();
                (character.input_coord, character.input_symbol)
            };

            let mut singularity_path = Path::new("singularity", 0.7);
            singularity_path.add_waypoint(input_coord);
            singularity_path.add_waypoint(center);
            singularity_path.ease = Some(easing::ease_in_out_sine);

            let mut explode_path = Path::new("explode", 0.9);
            explode_path.add_waypoint(center);
            explode_path.add_waypoint(input_coord);
            explode_path.ease = Some(easing::ease_out_cubic);

            let mut hole_scene = Scene::new("hole");
            let mut visual = CharacterVisual::new(symbol);
            visual.colors = Some(ColorPair::new(Some(hole_color), None));
            visual.formatted_symbol = visual.format_symbol();
            hole_scene.add_frame(visual, 1);

            let character = terminal.get_character_mut(*id).unwrap();
            character.motion.add_path(singularity_path);
            character.motion.add_path(explode_path);
            character.animation.add_scene(hole_scene);
        }

        // Phase 1: collapse toward the singularity.
        for id in &char_ids {
            let character = terminal.get_character_mut(*id).unwrap();
            character.motion.activate_path("singularity");
            character.animation.activate_scene("hole");
        }

        let mut max_steps_in = 0usize;
        for id in &char_ids {
            let character = terminal.get_character(*id).unwrap();
            if let Some(path) = character.motion.paths.get("singularity") {
                let total = path.total_distance();
                let steps = (total / path.speed).ceil() as usize;
                if steps > max_steps_in {
                    max_steps_in = steps;
                }
            }
        }

        let mut frames = Vec::new();
        for _ in 0..max_steps_in {
            terminal.step_animation();
            frames.push(terminal.render());
        }

        // Brief hold at the singularity.
        for _ in 0..5 {
            frames.push(terminal.render());
        }

        // Phase 2: release back to original positions.
        for id in &char_ids {
            let character = terminal.get_character_mut(*id).unwrap();
            character.motion.activate_path("explode");
            character.animation.activate_scene("default");
        }

        let mut max_steps_out = 0usize;
        for id in &char_ids {
            let character = terminal.get_character(*id).unwrap();
            if let Some(path) = character.motion.paths.get("explode") {
                let total = path.total_distance();
                let steps = (total / path.speed).ceil() as usize;
                if steps > max_steps_out {
                    max_steps_out = steps;
                }
            }
        }

        for _ in 0..max_steps_out {
            terminal.step_animation();
            frames.push(terminal.render());
        }

        frames
    }
}
