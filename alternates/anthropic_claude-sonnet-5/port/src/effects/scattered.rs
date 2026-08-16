//! Scattered: characters begin at random scattered coordinates on the
//! canvas and travel (each along its own eased path, tinted from a shared
//! gradient) back to their original input coordinate, settling into a
//! plain final color once arrived. Simplified port of
//! terminaltexteffects/effects/effect_scattered.py against the reduced
//! engine surface described in the crate skeleton.

use super::Effect;

use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::motion::Path;
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::{self, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Small deterministic xorshift32 PRNG so scatter positions are stable
/// across runs without depending on any crate outside this file.
struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Rng(seed.max(1))
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// Inclusive range [lo, hi].
    fn gen_range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let span = (hi - lo + 1) as u32;
        lo + (self.next_u32() % span) as i32
    }
}

pub struct Scattered;

impl Scattered {
    pub fn new() -> Self {
        Scattered
    }
}

impl Effect for Scattered {
    fn name(&self) -> &str {
        "scattered"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let width = (terminal.config.width.max(1) as i32) - 1;
        let height = (terminal.config.height.max(1) as i32) - 1;

        let gradient_stops = [
            Color::Rgb(255, 0, 128),
            Color::Rgb(0, 255, 255),
            Color::Rgb(255, 255, 0),
            Color::Rgb(255, 0, 128),
        ];
        let gradient = Gradient::new(&gradient_stops, 10);
        let gradient_len = gradient.len().max(1);

        let mut rng = Rng::new(0x9E37_79B9);

        let char_count = terminal.get_characters().len();
        let mut max_steps: u32 = 1;

        for idx in 0..char_count {
            let (target_coord, symbol) = {
                let ch = &terminal.get_characters()[idx];
                (ch.input_coord, ch.input_symbol)
            };

            let start_coord = Coord::new(rng.gen_range(0, width), rng.gen_range(0, height));

            let speed = 0.3 + ((idx % 7) as f64) * 0.15;
            let distance = geometry::distance(start_coord, target_coord);
            let steps_needed = (distance / speed).ceil().max(1.0) as u32;
            if steps_needed > max_steps {
                max_steps = steps_needed;
            }

            let color_idx = (idx * 7) % gradient_len;
            let travel_color = gradient.get(color_idx).unwrap_or(Color::Rgb(200, 200, 200));

            let ch = &mut terminal.get_characters_mut()[idx];
            ch.motion.current_pos = (start_coord.column as f64, start_coord.row as f64);
            ch.motion.current_coord = start_coord;

            let mut path = Path::new("home", speed);
            path.ease = Some(easing::ease_out_cubic);
            path.add_waypoint(start_coord);
            path.add_waypoint(target_coord);
            ch.motion.add_path(path);
            ch.motion.activate_path("home");

            let mut moving_visual = CharacterVisual::new(symbol);
            moving_visual.colors = Some(ColorPair::new(Some(travel_color), None));
            moving_visual.formatted_symbol = moving_visual.format_symbol();
            let mut moving_scene = Scene::new("moving");
            moving_scene.is_looping = true;
            moving_scene.add_frame(moving_visual, 1);
            ch.animation.add_scene(moving_scene);
            ch.animation.activate_scene("moving");

            let mut final_visual = CharacterVisual::new(symbol);
            final_visual.colors = Some(ColorPair::new(Some(Color::Rgb(255, 255, 255)), None));
            final_visual.formatted_symbol = final_visual.format_symbol();
            let mut final_scene = Scene::new("final");
            final_scene.is_looping = true;
            final_scene.add_frame(final_visual, 1);
            ch.animation.add_scene(final_scene);
        }

        let mut frames = Vec::new();
        frames.push(terminal.render());

        for _ in 0..max_steps {
            terminal.step_animation();
            frames.push(terminal.render());
        }

        for ch in terminal.get_characters_mut() {
            ch.animation.activate_scene("final");
        }

        for _ in 0..8 {
            terminal.step_animation();
            frames.push(terminal.render());
        }

        frames
    }
}
