use super::Effect;
use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::character::CharacterId;
use crate::engine::motion::Path;
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Build a `CharacterVisual` with the given symbol/colors and a freshly
/// computed `formatted_symbol`, mirroring the inline construction done by
/// `Animation::set_appearance`.
fn make_visual(symbol: char, colors: Option<ColorPair>) -> CharacterVisual {
    let mut visual = CharacterVisual::new(symbol);
    visual.colors = colors;
    visual.formatted_symbol = visual.format_symbol();
    visual
}

/// `smoke` — characters drift upward off the top of the canvas like rising
/// smoke, wavering left/right, brightening through a gray-to-white gradient
/// before dissipating into fainter symbols and finally blank space.
pub struct Smoke;

impl Smoke {
    pub fn new() -> Self {
        Smoke
    }
}

impl Effect for Smoke {
    fn name(&self) -> &str {
        "smoke"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let height = terminal.config.height as i32;

        // Shared smoke color spectrum: dark gray -> pale gray -> white.
        let stops = [
            Color::Rgb(90, 90, 90),
            Color::Rgb(190, 190, 195),
            Color::Rgb(255, 255, 255),
        ];
        let gradient = Gradient::new(&stops, 8);
        let last_color = *gradient.spectrum.last().unwrap_or(&Color::Rgb(255, 255, 255));

        let ids: Vec<CharacterId> = terminal.get_characters().iter().map(|c| c.id).collect();

        let mut max_ticks: u32 = 0;

        for id in ids {
            let (input_symbol, start) = {
                let ch = terminal.get_character(id).unwrap();
                (ch.input_symbol, ch.input_coord)
            };

            // Deterministic pseudo-variation derived from the character id,
            // standing in for the RNG the upstream effect uses (unavailable
            // in this skeleton).
            let phase = ((id as f64) * 0.618_033).sin();
            let phase2 = ((id as f64) * 0.318_309).cos();

            let speed = 0.25 + phase.abs() * 0.35;
            let wobble = 1 + (phase2.abs() * 2.0) as i32;

            let wp1 = Coord::new(start.column + wobble, start.row - 2);
            let wp2 = Coord::new(start.column - wobble, start.row - (height / 2 + 3));
            let wp3 = Coord::new(start.column + wobble, -(height) - 3);

            let mut path = Path::new("rise", speed);
            path.add_waypoint(start);
            path.add_waypoint(wp1);
            path.add_waypoint(wp2);
            path.add_waypoint(wp3);
            path.ease = Some(easing::ease_out_sine);

            let total_distance = path.total_distance();
            let path_ticks = if speed > 0.0 {
                (total_distance / speed).ceil() as u32
            } else {
                0
            };

            let mut scene = Scene::new("smoke");
            for color in gradient.spectrum.iter() {
                let visual = make_visual(input_symbol, Some(ColorPair::new(Some(*color), None)));
                scene.add_frame(visual, 2);
            }
            scene.add_frame(
                make_visual('*', Some(ColorPair::new(Some(last_color), None))),
                3,
            );
            scene.add_frame(
                make_visual('.', Some(ColorPair::new(Some(last_color), None))),
                3,
            );
            scene.add_frame(make_visual(' ', None), 1);

            let anim_ticks: u32 = scene.frames.iter().map(|f| f.duration).sum();

            let ch = terminal.get_character_mut(id).unwrap();
            ch.animation.add_scene(scene);
            ch.animation.activate_scene("smoke");
            ch.motion.add_path(path);
            ch.motion.activate_path("rise");

            let ticks_needed = path_ticks.max(anim_ticks);
            if ticks_needed > max_ticks {
                max_ticks = ticks_needed;
            }
        }

        let total_ticks = max_ticks + 5;

        let mut frames = Vec::with_capacity(total_ticks as usize + 1);
        frames.push(terminal.render());
        for _ in 0..total_ticks {
            terminal.step_animation();
            frames.push(terminal.render());
        }
        frames
    }
}
