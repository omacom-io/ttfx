//! Laser etch effect: a beam sweeps left-to-right across the canvas,
//! revealing each character as it passes and burning it in with a hot
//! laser color that cools down into its resting appearance over a few
//! ticks (mirrors the spirit of `terminaltexteffects/effects/effect_laseretch.py`,
//! adapted to the reduced engine surface available here: no event handler,
//! so scene activation is driven directly by the sweep loop instead of
//! motion-arrival callbacks).

use super::Effect;
use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, ColorPair, Gradient};

pub struct Laseretch {
    laser_color: Color,
    etched_color: Color,
    etch_ticks: u32,
}

impl Laseretch {
    pub fn new() -> Self {
        Laseretch {
            laser_color: Color::Rgb(255, 60, 40),
            etched_color: Color::Rgb(255, 255, 255),
            etch_ticks: 6,
        }
    }
}

impl Effect for Laseretch {
    fn name(&self) -> &str {
        "laseretch"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let width = terminal.config.width as u32;

        // Cooling gradient: hot laser color settling into the resting color.
        let gradient = Gradient::new(&[self.laser_color, self.etched_color], self.etch_ticks as usize);

        // Snapshot id/symbol/coord up front since we can't hold a borrow of
        // `terminal` while mutating it in the loop below.
        let characters: Vec<(u32, char, u32)> = terminal
            .get_characters()
            .iter()
            .map(|c| (c.id, c.input_symbol, c.input_coord.column.max(0) as u32))
            .collect();

        // Every character starts hidden and carries a pre-built "etch"
        // scene: a run of frames cooling from the laser color down to the
        // final resting color, held on the last frame once exhausted.
        for (id, symbol, _column) in &characters {
            terminal.set_character_visibility(*id, false);
            if let Some(character) = terminal.get_character_mut(*id) {
                let mut etch_scene = Scene::new("etch");
                for step in 0..gradient.len() {
                    let color = gradient.get(step).unwrap_or(self.etched_color);
                    let mut visual = CharacterVisual::new(*symbol);
                    visual.colors = Some(ColorPair::new(Some(color), None));
                    visual.formatted_symbol = visual.format_symbol();
                    etch_scene.add_frame(visual, 1);
                }
                character.animation.add_scene(etch_scene);
            }
        }

        let total_ticks = width + self.etch_ticks + 2;
        let mut frames = Vec::with_capacity(total_ticks as usize);

        for tick in 0..total_ticks {
            // The beam reaches column `tick`: reveal and ignite every
            // character sitting in that column across all rows.
            for (id, _symbol, column) in &characters {
                if *column == tick {
                    terminal.set_character_visibility(*id, true);
                    if let Some(character) = terminal.get_character_mut(*id) {
                        character.animation.activate_scene("etch");
                    }
                }
            }
            terminal.step_animation();
            frames.push(terminal.render());
        }

        frames
    }
}
