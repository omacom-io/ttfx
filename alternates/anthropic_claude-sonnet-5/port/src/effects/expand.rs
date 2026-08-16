use super::Effect;
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::{self, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Port of `terminaltexteffects/effects/effect_expand.py`.
///
/// Characters start collapsed at the canvas center and expand outward to
/// their input coordinates, eased with `in_out_cubic`, colored by a
/// horizontal gradient keyed on each character's input column.
pub struct Expand {
    steps: usize,
}

impl Expand {
    pub fn new() -> Self {
        Expand { steps: 30 }
    }
}

impl Effect for Expand {
    fn name(&self) -> &str {
        "expand"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let width = terminal.canvas.width.max(1);
        let height = terminal.canvas.height.max(1);
        let center = Coord::new((width as i32) / 2, (height as i32) / 2);

        // Horizontal gradient spectrum, one color band per canvas column,
        // mirroring the colorful reveal produced by the Python effect's
        // graphics.Gradient usage.
        let gradient = Gradient::new(
            &[
                Color::Rgb(0, 255, 255),
                Color::Rgb(255, 0, 255),
                Color::Rgb(255, 255, 0),
            ],
            width,
        );

        // Snapshot each character's destination (its input coord) and the
        // color it should carry, based on that column.
        let ids: Vec<u32> = terminal.get_characters().iter().map(|c| c.id).collect();
        let mut destinations: Vec<(u32, Coord, Color)> = Vec::with_capacity(ids.len());
        for &id in &ids {
            let character = terminal.get_character(id).unwrap();
            let input_coord = character.input_coord;
            let column = input_coord.column.max(0) as usize;
            let idx = column.min(gradient.len().saturating_sub(1));
            let color = gradient.get(idx).unwrap_or(Color::Rgb(255, 255, 255));
            destinations.push((id, input_coord, color));
        }

        // Collapse every character to the canvas center and give it its
        // colored appearance up front (a fixed, non-animated Scene, so we
        // never call step_animation and clobber the color).
        for &(id, _, color) in &destinations {
            if let Some(character) = terminal.get_character_mut(id) {
                character.motion.current_coord = center;
                character.motion.current_pos = (center.column as f64, center.row as f64);
                let colors = ColorPair::new(Some(color), None);
                character.animation.set_appearance(character.input_symbol, Some(colors));
            }
        }

        let mut frames = Vec::with_capacity(self.steps + 1);
        frames.push(terminal.render());

        for step in 1..=self.steps {
            let t = (step as f64 / self.steps as f64).clamp(0.0, 1.0);
            let eased = easing::ease_in_out_cubic(t);
            for &(id, dest, _) in &destinations {
                if let Some(character) = terminal.get_character_mut(id) {
                    let (x, y) = geometry::lerp(center, dest, eased);
                    character.motion.current_pos = (x, y);
                    character.motion.current_coord = Coord::new(x.round() as i32, y.round() as i32);
                }
            }
            frames.push(terminal.render());
        }

        frames
    }
}
