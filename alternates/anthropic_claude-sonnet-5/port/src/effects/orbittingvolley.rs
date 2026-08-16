use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::terminal::Terminal;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

use super::Effect;

/// Simplified port of `effect_orbittingvolley.py`: characters are grouped
/// into a "left cannon" and "right cannon" volley based on which half of the
/// canvas they land in, staggered by their distance from the launch corner,
/// and colored through a launch gradient -> impact flash -> resting color
/// sequence before settling into their final input position.
pub struct Orbittingvolley;

impl Orbittingvolley {
    pub fn new() -> Self {
        Orbittingvolley
    }
}

fn colored_visual(symbol: char, color: Color) -> CharacterVisual {
    let mut visual = CharacterVisual::new(symbol);
    visual.colors = Some(ColorPair::new(Some(color), None));
    visual.formatted_symbol = visual.format_symbol();
    visual
}

impl Effect for Orbittingvolley {
    fn name(&self) -> &str {
        "orbittingvolley"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let width = terminal.config.width as i32;

        let left_gradient = Gradient::new(
            &[Color::Rgb(255, 140, 0), Color::Rgb(255, 255, 0), Color::Rgb(255, 80, 0)],
            4,
        );
        let right_gradient = Gradient::new(
            &[Color::Rgb(0, 150, 255), Color::Rgb(0, 255, 255), Color::Rgb(80, 0, 255)],
            4,
        );
        let impact_color = Color::Rgb(255, 255, 255);
        let resting_color = Color::Rgb(210, 210, 210);

        let char_count = terminal.get_characters().len();
        let mut delays = vec![0u32; char_count];

        for character in terminal.get_characters() {
            let idx = character.id as usize;
            let is_left = character.input_coord.column < width / 2;
            let launch = if is_left { Coord::new(0, 0) } else { Coord::new(width - 1, 0) };
            let dist = ((character.input_coord.column - launch.column).abs()
                + (character.input_coord.row - launch.row).abs()) as u32;
            delays[idx] = (dist / 2).clamp(1, 30);
        }

        let mut max_ticks: u32 = 0;

        for character in terminal.get_characters_mut() {
            let idx = character.id as usize;
            let is_left = character.input_coord.column < width / 2;
            let gradient = if is_left { &left_gradient } else { &right_gradient };
            let symbol = character.input_symbol;
            let delay = delays[idx];

            let mut scene = Scene::new("volley");
            let charge_color = gradient.get(0).unwrap_or(Color::Rgb(255, 255, 255));
            scene.add_frame(colored_visual(symbol, charge_color), delay);
            for i in 0..gradient.len() {
                if let Some(c) = gradient.get(i) {
                    scene.add_frame(colored_visual(symbol, c), 1);
                }
            }
            scene.add_frame(colored_visual(symbol, impact_color), 2);
            scene.add_frame(colored_visual(symbol, resting_color), 1);

            let total = delay + gradient.len() as u32 + 2 + 1;
            if total > max_ticks {
                max_ticks = total;
            }

            character.animation.add_scene(scene);
            character.animation.activate_scene("volley");
        }

        let extra_hold = 10;
        let mut out_frames = Vec::new();
        for _ in 0..(max_ticks + extra_hold) {
            terminal.step_animation();
            out_frames.push(terminal.render());
        }
        out_frames
    }
}
