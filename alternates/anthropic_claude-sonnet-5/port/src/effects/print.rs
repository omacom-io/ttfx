
use super::Effect;
use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair};

/// The `print` effect: characters appear to be printed onto the canvas one
/// at a time, row by row, rising up from below their final position with a
/// hot printhead flash that cools down to the character's resting color.
///
/// This mirrors the spirit of `terminaltexteffects/effects/effect_print.py`
/// within the constraints of the ported engine: because the skeleton's
/// `Motion::step` only resolves correctly for single-waypoint (static)
/// paths, the rise animation is computed directly per-tick here rather than
/// via `Motion::new_path`/`activate_path`, using the same row-major
/// character ordering the terminal arena already produces as the print
/// (typewriter) order.
pub struct Print {
    name: String,
}

impl Print {
    pub fn new() -> Self {
        Print { name: "print".to_string() }
    }
}

impl Effect for Print {
    fn name(&self) -> &str {
        &self.name
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let height = terminal.config.height as i32;

        // Printhead heat falloff: a bright flash cooling to a mid gray
        // before settling to the character's plain resting appearance.
        let flash_color = Color::Rgb(255, 255, 255);
        let mid_color = Color::Rgb(180, 180, 180);

        // Hide every character until the printhead reaches it, and register
        // the per-character print scene (flash -> cool -> settle).
        for character in terminal.get_characters_mut() {
            character.set_visibility(false);

            let symbol = character.input_symbol;

            let mut flash_visual = CharacterVisual::new(symbol);
            flash_visual.bold = true;
            flash_visual.colors = Some(ColorPair::new(Some(flash_color), None));
            flash_visual.formatted_symbol = flash_visual.format_symbol();

            let mut mid_visual = CharacterVisual::new(symbol);
            mid_visual.colors = Some(ColorPair::new(Some(mid_color), None));
            mid_visual.formatted_symbol = mid_visual.format_symbol();

            let final_visual = CharacterVisual::new(symbol);

            let mut scene = Scene::new("print");
            scene.add_frame(flash_visual, 2);
            scene.add_frame(mid_visual, 2);
            scene.add_frame(final_visual, 1);

            character.animation.add_scene(scene);
        }

        // The terminal's arena is already built in row-major (top-to-bottom,
        // left-to-right) order, matching a printer's typing order, so a
        // character's arena index doubles as its reveal tick.
        let num_chars = terminal.get_characters().len();
        let rise_ticks: i32 = (height + 4).max(4);
        let settle_buffer: usize = rise_ticks as usize + 4;
        let total_ticks = num_chars + settle_buffer;

        let mut frames = Vec::with_capacity(total_ticks);

        for tick in 0..total_ticks {
            for (idx, character) in terminal.get_characters_mut().iter_mut().enumerate() {
                if tick < idx {
                    continue;
                }

                if !character.visible {
                    character.set_visibility(true);
                    character.animation.activate_scene("print");
                }
                character.animation.step_animation();

                let elapsed = (tick - idx) as f64;
                let t = (elapsed / rise_ticks as f64).min(1.0);
                let eased = easing::ease_out_cubic(t);
                let start_row = height as f64;
                let final_row = character.input_coord.row as f64;
                let row = start_row + (final_row - start_row) * eased;
                character.motion.current_coord =
                    Coord::new(character.input_coord.column, row.round() as i32);
            }
            frames.push(terminal.render());
        }

        frames
    }
}
