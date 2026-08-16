use super::Effect;

use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Beams: characters light up along a diagonal sweep, glowing through a
/// short color gradient before settling to their plain symbol. A simplified
/// stand-in for terminaltexteffects/effects/effect_beams.py's traveling
/// light-beam illumination, built entirely on the engine primitives above
/// (no dedicated beam/group objects, no RNG available in this port yet).
pub struct Beams {
    name: String,
}

impl Beams {
    pub fn new() -> Self {
        Beams { name: "beams".to_string() }
    }
}

/// Build the "beam" scene for a character: a short glowing gradient fade
/// followed by a settled, uncolored final frame that the (non-looping)
/// scene holds on indefinitely once reached.
fn build_beam_scene(symbol: char, spectrum: &[Color]) -> Scene {
    let mut scene = Scene::new("beam");
    for color in spectrum {
        let mut visual = CharacterVisual::new(symbol);
        visual.colors = Some(ColorPair::new(Some(*color), None));
        visual.formatted_symbol = visual.format_symbol();
        scene.add_frame(visual, 1);
    }
    // Settled appearance: plain symbol, no color override.
    let final_visual = CharacterVisual::new(symbol);
    scene.add_frame(final_visual, 1);
    scene.is_looping = false;
    scene
}

impl Effect for Beams {
    fn name(&self) -> &str {
        &self.name
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);

        // Fixed beam color palettes (no rng module available in this port),
        // varied per character by position for visual texture.
        let palettes: [[Color; 2]; 3] = [
            [Color::Rgb(255, 255, 150), Color::Rgb(80, 160, 255)],
            [Color::Rgb(150, 255, 255), Color::Rgb(255, 120, 200)],
            [Color::Rgb(200, 255, 150), Color::Rgb(120, 120, 255)],
        ];
        let gradient_steps: usize = 5;

        // Precompute each character's activation delay: a diagonal sweep
        // based on (row + column), and hide everything until the beam
        // reaches it.
        let mut delays: Vec<(u32, u32)> = Vec::new();
        for character in terminal.get_characters().iter() {
            let delay = (character.input_coord.column + character.input_coord.row).max(0) as u32;
            delays.push((character.id, delay));
        }

        for character in terminal.get_characters_mut().iter_mut() {
            character.set_visibility(false);
            let palette_index =
                ((character.input_coord.row + character.input_coord.column).max(0) as usize) % palettes.len();
            let palette = &palettes[palette_index];
            let gradient = Gradient::new(palette, gradient_steps);
            let scene = build_beam_scene(character.input_symbol, &gradient.spectrum);
            character.animation.add_scene(scene);
        }

        let max_delay = delays.iter().map(|(_, d)| *d).max().unwrap_or(0);
        let settle_ticks = gradient_steps as u32 + 2;
        let total_ticks = max_delay + settle_ticks + 3;

        let mut frames = Vec::with_capacity(total_ticks as usize);
        for tick in 0..total_ticks {
            for (id, delay) in &delays {
                if *delay == tick {
                    if let Some(character) = terminal.get_character_mut(*id) {
                        character.set_visibility(true);
                        character.animation.activate_scene("beam");
                    }
                }
            }
            terminal.step_animation();
            frames.push(terminal.render());
        }

        frames
    }
}
