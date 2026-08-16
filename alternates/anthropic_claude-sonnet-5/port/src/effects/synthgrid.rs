
use super::Effect;
use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::terminal::Terminal;
use crate::utils::geometry::{self, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Port of `terminaltexteffects/effects/effect_synthgrid.py`, simplified to
/// the primitives exposed by this crate's engine: rather than building a
/// separate overlay grid of characters (upstream's `add_character` calls),
/// every canvas cell's own character plays a three-stage scene — a blank
/// hold (staggered by distance from center, mirroring the grid's outward
/// draw order), a "grid line" glyph tinted from a gradient, and finally its
/// resolved input symbol.
pub struct Synthgrid;

impl Synthgrid {
    pub fn new() -> Self {
        Synthgrid
    }
}

impl Effect for Synthgrid {
    fn name(&self) -> &str {
        "synthgrid"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let width = terminal.config.width as i32;
        let height = terminal.config.height as i32;
        let center = Coord::new(width / 2, height / 2);

        let grid_gradient = Gradient::new(&[Color::Rgb(0, 255, 255), Color::Rgb(255, 0, 255)], 10);

        let mut max_dist = 0.0f64;
        for character in terminal.get_characters() {
            let d = geometry::distance(center, character.input_coord);
            if d > max_dist {
                max_dist = d;
            }
        }
        if max_dist <= 0.0 {
            max_dist = 1.0;
        }

        let chars_len = terminal.get_characters().len();
        const GRID_DURATION: u32 = 6;
        const STAGGER_TICKS: f64 = 15.0;

        for idx in 0..chars_len {
            let (input_symbol, input_coord) = {
                let character = &terminal.get_characters()[idx];
                (character.input_symbol, character.input_coord)
            };

            let dist = geometry::distance(center, input_coord);
            let delay = ((dist / max_dist) * STAGGER_TICKS).round() as u32 + 1;

            let gradient_index = ((input_coord.column + input_coord.row).unsigned_abs() as usize)
                % grid_gradient.len().max(1);
            let grid_color = grid_gradient
                .get(gradient_index)
                .unwrap_or(Color::Rgb(255, 255, 255));
            let grid_symbol = if (input_coord.column + input_coord.row) % 2 == 0 {
                '+'
            } else {
                '-'
            };

            let mut scene = Scene::new("synthgrid");

            let blank_visual = CharacterVisual::new(' ');
            scene.add_frame(blank_visual, delay);

            let mut grid_visual = CharacterVisual::new(grid_symbol);
            grid_visual.colors = Some(ColorPair::new(Some(grid_color), None));
            grid_visual.formatted_symbol = grid_visual.format_symbol();
            scene.add_frame(grid_visual, GRID_DURATION);

            let final_visual = CharacterVisual::new(input_symbol);
            scene.add_frame(final_visual, 1_000);

            let character = terminal
                .get_character_mut(idx as u32)
                .expect("index within arena bounds");
            character.animation.add_scene(scene);
            character.animation.activate_scene("synthgrid");
            character.set_visibility(true);
        }

        let total_ticks = STAGGER_TICKS as u32 + GRID_DURATION + 5;

        let mut frames = Vec::new();
        frames.push(terminal.render());
        for _ in 0..total_ticks {
            terminal.step_animation();
            frames.push(terminal.render());
        }
        frames
    }
}
