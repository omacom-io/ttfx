use super::Effect;

use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Cycles every character's foreground color through a rainbow spectrum,
/// skewing the starting phase per-character by its (column + row) so the
/// shift reads as a diagonal sweep across the text, then holds on the
/// final color once the cycle completes (mirrors the shape of
/// `terminaltexteffects/effects/effect_colorshift.py`'s traveling gradient
/// shift, simplified to the primitives available in this engine skeleton).
pub struct Colorshift;

impl Colorshift {
    pub fn new() -> Self {
        Colorshift
    }
}

impl Effect for Colorshift {
    fn name(&self) -> &str {
        "colorshift"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);

        // Rainbow stops, looping back to the starting hue so the cycle is
        // seamless.
        let stops = [
            Color::Rgb(255, 0, 0),
            Color::Rgb(255, 255, 0),
            Color::Rgb(0, 255, 0),
            Color::Rgb(0, 255, 255),
            Color::Rgb(0, 0, 255),
            Color::Rgb(255, 0, 255),
            Color::Rgb(255, 0, 0),
        ];
        let steps_per_segment = 4;
        let gradient = Gradient::new(&stops, steps_per_segment);
        let spectrum = gradient.spectrum;
        let spectrum_len = spectrum.len().max(1);

        for character in terminal.get_characters_mut() {
            let offset =
                ((character.input_coord.column + character.input_coord.row).max(0) as usize) % spectrum_len;

            let mut scene = Scene::new("colorshift");
            for i in 0..spectrum_len {
                let color = spectrum[(i + offset) % spectrum_len];
                let mut visual = CharacterVisual::new(character.input_symbol);
                visual.colors = Some(ColorPair::new(Some(color), None));
                visual.formatted_symbol = visual.format_symbol();
                scene.add_frame(visual, 1);
            }
            character.animation.add_scene(scene);
            character.animation.activate_scene("colorshift");
        }

        let mut frames = Vec::with_capacity(spectrum_len);
        frames.push(terminal.render());
        for _ in 1..spectrum_len {
            terminal.step_animation();
            frames.push(terminal.render());
        }
        frames
    }
}
