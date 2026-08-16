//! Burn effect: characters begin coated in ash, ignite from the bottom row
//! upward through a fire-color gradient, and settle into their revealed
//! (uncolored) final appearance. Approximates
//! `terminaltexteffects/effects/effect_burn.py`'s "burn upward, reveal text"
//! shape using only the primitives exposed by this port's engine (no RNG is
//! available yet, so the per-character ignition delay is a deterministic
//! function of position rather than randomized jitter).

use super::Effect;

use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, ColorPair};

pub struct Burn;

impl Burn {
    pub fn new() -> Self {
        Burn
    }
}

impl Effect for Burn {
    fn name(&self) -> &str {
        "burn"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let height = terminal.config.height;

        // Fire progression from cool ash through to a white-hot flash,
        // after which the character settles into its plain revealed symbol.
        let fire_colors: Vec<Color> = vec![
            Color::Rgb(40, 10, 10),
            Color::Rgb(90, 20, 10),
            Color::Rgb(140, 30, 10),
            Color::Rgb(180, 60, 10),
            Color::Rgb(220, 100, 10),
            Color::Rgb(255, 140, 20),
            Color::Rgb(255, 190, 60),
            Color::Rgb(255, 230, 120),
            Color::Rgb(255, 255, 255),
        ];

        let delay_per_row: i32 = 3;
        let mut max_total: usize = 1;

        for character in terminal.get_characters_mut() {
            let symbol = character.input_symbol;
            if symbol == ' ' {
                continue;
            }

            let row = character.input_coord.row;
            let column = character.input_coord.column;
            // Bottom rows (largest row value) ignite first; ignition spreads
            // upward. A small column-based offset keeps the wavefront from
            // being perfectly flat.
            let delay = (((height as i32 - 1 - row) * delay_per_row) + (column % 3)).max(0) as u32;

            let mut scene = Scene::new("burn");

            if delay > 0 {
                let mut ash_visual = CharacterVisual::new(symbol);
                ash_visual.colors = Some(ColorPair::new(Some(fire_colors[0]), None));
                ash_visual.formatted_symbol = ash_visual.format_symbol();
                for _ in 0..delay {
                    scene.add_frame(ash_visual.clone(), 1);
                }
            }

            for color in &fire_colors {
                let mut visual = CharacterVisual::new(symbol);
                visual.colors = Some(ColorPair::new(Some(*color), None));
                visual.formatted_symbol = visual.format_symbol();
                scene.add_frame(visual, 1);
            }

            // Revealed final state: plain symbol, no fire tint.
            let final_visual = CharacterVisual::new(symbol);
            scene.add_frame(final_visual, 1);

            let total_len = delay as usize + fire_colors.len() + 1;
            if total_len > max_total {
                max_total = total_len;
            }

            character.animation.add_scene(scene);
            character.animation.activate_scene("burn");
        }

        let mut frames = Vec::with_capacity(max_total + 1);
        frames.push(terminal.render());
        for _ in 0..max_total {
            terminal.step_animation();
            frames.push(terminal.render());
        }
        frames
    }
}
