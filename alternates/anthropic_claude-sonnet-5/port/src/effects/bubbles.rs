//! Bubbles effect (mirrors terminaltexteffects/effects/effect_bubbles.py).
//!
//! Characters are grouped into small "bubbles" that float up from below the
//! canvas and settle into their input position, each bubble sharing a single
//! color drawn from a fixed palette (standing in for the upstream rainbow /
//! random bubble-color config in this simplified engine).

use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::motion::Path;
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair};

use super::Effect;

/// Fixed bubble color palette, standing in for upstream's randomized bubble
/// coloring (`random.choice` over a gradient spectrum) since this skeleton
/// has no `rng` module wired up yet.
const BUBBLE_COLORS: [Color; 6] = [
    Color::Rgb(0xd0, 0xff, 0xf5),
    Color::Rgb(0x8c, 0xe8, 0xff),
    Color::Rgb(0x5a, 0xc8, 0xfa),
    Color::Rgb(0x39, 0x9e, 0xf5),
    Color::Rgb(0x6a, 0x7c, 0xff),
    Color::Rgb(0xb0, 0x9a, 0xff),
];

/// How many consecutive (non-space) characters share a single bubble/color.
const BUBBLE_GROUP_SIZE: usize = 4;

/// Extra frames to render once every bubble has settled, so the final
/// appearance holds briefly rather than cutting off mid-motion.
const HOLD_FRAMES: usize = 15;

/// A hard cap on simulated ticks so pathological inputs can't produce an
/// unbounded frame list.
const MAX_FRAMES: usize = 400;

#[derive(Debug, Clone)]
pub struct Bubbles {
    pub bubble_speed: f64,
}

impl Bubbles {
    pub fn new() -> Self {
        Bubbles { bubble_speed: 0.2 }
    }
}

impl Default for Bubbles {
    fn default() -> Self {
        Bubbles::new()
    }
}

impl Effect for Bubbles {
    fn name(&self) -> &str {
        "bubbles"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let bottom_row = terminal.config.height as f64 + 1.0;

        let mut bubble_index: usize = 0;
        let mut max_steps: usize = 0;

        for character in terminal.get_characters_mut() {
            if character.input_symbol == ' ' {
                continue;
            }

            let color = BUBBLE_COLORS[(bubble_index / BUBBLE_GROUP_SIZE) % BUBBLE_COLORS.len()];
            bubble_index += 1;

            // Bubble appearance: the input symbol tinted with the bubble's
            // shared color.
            let mut visual = CharacterVisual::new(character.input_symbol);
            visual.colors = Some(ColorPair::new(Some(color), None));
            visual.formatted_symbol = visual.format_symbol();

            let mut scene = Scene::new("bubble");
            scene.add_frame(visual, 1);
            character.animation.add_scene(scene);
            character.animation.activate_scene("bubble");

            // Motion: rise from below the canvas up to the character's
            // input position.
            let start_coord = Coord::new(character.input_coord.column, bottom_row.round() as i32);
            let mut path = Path::new("rise", self.bubble_speed.max(0.001));
            path.ease = Some(easing::ease_out_quad);
            path.add_waypoint(start_coord);
            path.add_waypoint(character.input_coord);

            let steps = if path.speed > 0.0 {
                (path.total_distance() / path.speed).ceil().max(1.0) as usize
            } else {
                1
            };
            max_steps = max_steps.max(steps);

            character.motion.add_path(path);
            character.motion.activate_path("rise");
        }

        let total_frames = (max_steps + HOLD_FRAMES).min(MAX_FRAMES);

        let mut frames = Vec::with_capacity(total_frames + 1);
        frames.push(terminal.render());
        for _ in 0..total_frames {
            terminal.step_animation();
            frames.push(terminal.render());
        }
        frames
    }
}
