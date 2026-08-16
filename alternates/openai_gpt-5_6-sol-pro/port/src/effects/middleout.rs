
use super::Effect;
use crate::engine::animation::{CharacterVisual, Frame, Scene};
use crate::engine::motion::{Path, Waypoint};
use crate::engine::terminal::Terminal;
use crate::utils::easing::in_out_sine;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Style};

const STARTING_COLOR: Color = Color::rgb(255, 255, 255);
const CENTER_MOVEMENT_SPEED: f64 = 0.35;
const FULL_MOVEMENT_SPEED: f64 = 0.35;
const COLOR_STEPS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharacterPhase {
    Center,
    Full,
    Done,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Middleout;

impl Middleout {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Middleout {
    fn name(&self) -> &str {
        "middleout"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);

        if terminal.characters().is_empty() {
            return Vec::new();
        }

        let width = terminal.canvas().width();
        let height = terminal.canvas().height();
        let center = Coord::new(
            (width.saturating_sub(1) / 2) as i32,
            (height.saturating_sub(1) / 2) as i32,
        );

        let targets = terminal
            .characters()
            .iter()
            .map(|character| character.position)
            .collect::<Vec<_>>();
        let final_colors = targets
            .iter()
            .map(|target| final_color_for_row(target.y, height))
            .collect::<Vec<_>>();
        let mut phases = vec![CharacterPhase::Center; targets.len()];

        for (index, character) in terminal.characters_mut().iter_mut().enumerate() {
            let target = targets[index];
            let middle_coord = Coord::new(center.x, target.y);

            character.set_position(center);
            character.set_appearance(
                character.input_symbol,
                Style::default().with_foreground(STARTING_COLOR),
            );

            let mut center_path = Path::with_waypoints(
                vec![Waypoint::new(center), Waypoint::new(middle_coord)],
                CENTER_MOVEMENT_SPEED,
            );
            center_path.set_easing(in_out_sine);
            character.motion.activate_path(center_path);
        }

        let mut frames = Vec::new();

        while phases
            .iter()
            .any(|phase| *phase != CharacterPhase::Done)
        {
            terminal.step();

            for (index, character) in terminal.characters_mut().iter_mut().enumerate() {
                let motion_active = character
                    .motion
                    .active_path()
                    .is_some_and(|path| path.is_active());

                match phases[index] {
                    CharacterPhase::Center if !motion_active => {
                        let mut full_path = Path::with_waypoints(
                            vec![
                                Waypoint::new(character.position),
                                Waypoint::new(targets[index]),
                            ],
                            FULL_MOVEMENT_SPEED,
                        );
                        full_path.set_easing(in_out_sine);
                        character.motion.activate_path(full_path);

                        let scene =
                            color_scene(character.input_symbol, final_colors[index], COLOR_STEPS);
                        character.animation.activate_scene(scene);
                        phases[index] = CharacterPhase::Full;
                    }
                    CharacterPhase::Full => {
                        let animation_active = character
                            .animation
                            .active_scene()
                            .is_some_and(|scene| scene.is_active());

                        if !motion_active && !animation_active {
                            character.set_position(targets[index]);
                            character.set_appearance(
                                character.input_symbol,
                                Style::default().with_foreground(final_colors[index]),
                            );
                            phases[index] = CharacterPhase::Done;
                        }
                    }
                    CharacterPhase::Center
                    | CharacterPhase::Full
                    | CharacterPhase::Done => {}
                }
            }

            frames.push(terminal.render_frame());
        }

        frames
    }
}

fn color_scene(symbol: char, final_color: Color, steps: usize) -> Scene {
    let steps = steps.max(1);
    let mut scene = Scene::new(false);

    for index in 0..steps {
        let progress = if steps == 1 {
            1.0
        } else {
            index as f64 / (steps - 1) as f64
        };
        let color = interpolate_color(STARTING_COLOR, final_color, progress);
        let style = Style::default().with_foreground(color);
        scene.add_frame(Frame::new(CharacterVisual::new(symbol, style), 1));
    }

    scene
}

fn final_color_for_row(row: i32, height: usize) -> Color {
    const STOPS: [Color; 3] = [
        Color::rgb(138, 0, 138),
        Color::rgb(0, 209, 255),
        Color::rgb(255, 255, 255),
    ];

    let progress = if height <= 1 {
        0.0
    } else {
        1.0 - (row.max(0) as f64 / (height - 1) as f64)
    }
    .clamp(0.0, 1.0);

    let scaled = progress * (STOPS.len() - 1) as f64;
    let segment = (scaled.floor() as usize).min(STOPS.len() - 2);
    interpolate_color(STOPS[segment], STOPS[segment + 1], scaled - segment as f64)
}

fn interpolate_color(start: Color, end: Color, progress: f64) -> Color {
    let (start_r, start_g, start_b) = rgb_components(start);
    let (end_r, end_g, end_b) = rgb_components(end);
    let progress = progress.clamp(0.0, 1.0);

    Color::rgb(
        interpolate_channel(start_r, end_r, progress),
        interpolate_channel(start_g, end_g, progress),
        interpolate_channel(start_b, end_b, progress),
    )
}

fn rgb_components(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb { r, g, b } => (r, g, b),
        Color::Ansi(value) => (value, value, value),
    }
}

fn interpolate_channel(start: u8, end: u8, progress: f64) -> u8 {
    (f64::from(start) + (f64::from(end) - f64::from(start)) * progress)
        .round()
        .clamp(0.0, 255.0) as u8
}
