//! Waves: a traveling color-gradient wave that sweeps across the text,
//! phase-shifted per column so the wave appears to move left-to-right,
//! before each character settles back to its original appearance.
//! Mirrors the visual intent of terminaltexteffects/effects/effect_waves.py,
//! adapted to the engine primitives available in this port (Motion's
//! multi-waypoint stepping is not usable for real displacement here, so the
//! traveling-wave illusion is produced entirely through animation scenes).

use super::Effect;
use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::character::CharacterId;
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, ColorPair, Gradient};

pub struct Waves {
    wave_gradient_stops: Vec<Color>,
    wave_gradient_steps: usize,
    wave_count: usize,
}

impl Waves {
    pub fn new() -> Self {
        Waves {
            wave_gradient_stops: vec![
                Color::Rgb(0, 0, 128),
                Color::Rgb(0, 128, 255),
                Color::Rgb(0, 255, 255),
                Color::Rgb(0, 128, 255),
                Color::Rgb(0, 0, 128),
            ],
            wave_gradient_steps: 6,
            wave_count: 2,
        }
    }
}

impl Effect for Waves {
    fn name(&self) -> &str {
        "waves"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let gradient = Gradient::new(&self.wave_gradient_stops, self.wave_gradient_steps);
        let gradient_len = gradient.len().max(1);

        let width = terminal.config.width.max(1);

        // One full sweep of the gradient repeated `wave_count` times gives
        // the duration each character spends actively "waving" before it
        // settles back to its default appearance.
        let sweep_len = gradient_len * self.wave_count.max(1);
        let total_ticks = width + sweep_len;

        let char_ids: Vec<CharacterId> = terminal.get_characters().iter().map(|c| c.id).collect();

        // Build a phase-shifted "wave" scene per character: the color index
        // at step `s` is offset by the character's column so the gradient
        // appears to travel across the canvas as ticks advance.
        for id in &char_ids {
            let (column, symbol) = {
                let character = terminal.get_character(*id).unwrap();
                (character.input_coord.column.max(0) as usize, character.input_symbol)
            };

            let mut wave_scene = Scene::new("wave");
            for step in 0..sweep_len {
                let index = (step + column) % gradient_len;
                let color = gradient.get(index).unwrap_or(Color::Rgb(255, 255, 255));
                let mut visual = CharacterVisual::new(symbol);
                visual.colors = Some(ColorPair::new(Some(color), None));
                visual.formatted_symbol = visual.format_symbol();
                wave_scene.add_frame(visual, 1);
            }
            wave_scene.is_looping = false;

            let character = terminal.get_character_mut(*id).unwrap();
            character.animation.add_scene(wave_scene);
        }

        let mut frames = Vec::with_capacity(total_ticks);
        for tick in 0..total_ticks {
            for id in &char_ids {
                let column = terminal.get_character(*id).unwrap().input_coord.column.max(0) as usize;
                if tick == column {
                    let character = terminal.get_character_mut(*id).unwrap();
                    character.animation.activate_scene("wave");
                } else if tick == column + sweep_len {
                    let character = terminal.get_character_mut(*id).unwrap();
                    character.animation.activate_scene("default");
                }
            }
            terminal.step_animation();
            frames.push(terminal.render());
        }

        frames
    }
}
