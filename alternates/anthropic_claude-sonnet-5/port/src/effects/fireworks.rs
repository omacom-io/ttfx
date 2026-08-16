//! Fireworks effect: characters launch from the bottom of the canvas like
//! sparks, burst radially outward at a random point near the top, then fall
//! under "gravity" into their final input position. Mirrors the spirit of
//! `terminaltexteffects/effects/effect_fireworks.py`, simplified to the
//! primitives available in this port's engine (no event/particle system, so
//! phase transitions are driven directly by the effect's own frame loop).

use super::Effect;

use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::character::CharacterId;
use crate::engine::motion::{Path, Segment, Waypoint};
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::{distance, Coord};
use crate::utils::graphics::{Color, ColorPair};

const FIREWORK_COLORS: [Color; 7] = [
    Color::Rgb(255, 0, 0),
    Color::Rgb(255, 165, 0),
    Color::Rgb(255, 255, 0),
    Color::Rgb(0, 255, 0),
    Color::Rgb(0, 255, 255),
    Color::Rgb(80, 120, 255),
    Color::Rgb(255, 0, 255),
];

const SHELL_SIZE: usize = 10;
const STAGGER: i64 = 14;

/// Small deterministic PRNG (xorshift64*) so the effect is reproducible
/// without depending on an external RNG crate or the `rng.rs` module
/// mentioned in the plan (not present among the available files).
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Rng { state: seed.max(1) }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Integer in `[lo, hi)`.
    fn gen_range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let range = (hi - lo) as u64;
        lo + (self.next_u64() % range) as i32
    }

    fn gen_f64(&mut self) -> f64 {
        (self.next_u64() % 1_000_000) as f64 / 1_000_000.0
    }

    fn choice_color(&mut self, colors: &[Color]) -> Color {
        colors[self.gen_range(0, colors.len() as i32) as usize]
    }
}

fn dim(color: Color) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as f64 * 0.45) as u8,
            (g as f64 * 0.45) as u8,
            (b as f64 * 0.45) as u8,
        ),
        Color::Ansi256(n) => Color::Ansi256(n),
    }
}

fn single_frame_scene(id: &str, symbol: char, color: Color) -> Scene {
    let mut visual = CharacterVisual::new(symbol);
    visual.colors = Some(ColorPair::new(Some(color), None));
    visual.formatted_symbol = visual.format_symbol();
    let mut scene = Scene::new(id);
    scene.add_frame(visual, 1);
    scene.is_looping = true;
    scene
}

/// Build a single-segment path directly from waypoints, bypassing
/// `Path::add_waypoint`'s zero-length anchor segment (which would otherwise
/// always short-circuit `Motion::step`'s segment walk on the anchor).
fn build_path(id: &str, speed: f64, ease: easing::EasingFunction, start: Coord, end: Coord) -> Path {
    let mut path = Path::new(id, speed);
    path.ease = Some(ease);
    path.segments.push(Segment::new(Waypoint::new(start), Waypoint::new(end)));
    path
}

struct CharPlan {
    id: CharacterId,
    launch_start: Coord,
    burst_coord: Coord,
    peak_coord: Coord,
    final_coord: Coord,
    color: Color,
    shell_start: i64,
    launch_steps: i64,
    burst_steps: i64,
    fall_steps: i64,
}

pub struct Fireworks;

impl Fireworks {
    pub fn new() -> Self {
        Fireworks
    }
}

impl Effect for Fireworks {
    fn name(&self) -> &str {
        "fireworks"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let width = terminal.config.width as i32;
        let height = terminal.config.height as i32;
        let top = 0i32;
        let bottom = (height - 1).max(0);

        let mut rng = Rng::new(0x9E37_79B9_7F4A_7C15);

        // Non-space characters, in arena (row-major allocation) order.
        let mut ids: Vec<CharacterId> = Vec::new();
        for character in terminal.get_characters() {
            if character.input_symbol != ' ' {
                ids.push(character.id);
            }
        }

        let mut plans: Vec<CharPlan> = Vec::new();

        for (shell_idx, chunk) in ids.chunks(SHELL_SIZE).enumerate() {
            let mut sum_col = 0i32;
            for &id in chunk {
                if let Some(character) = terminal.get_character(id) {
                    sum_col += character.input_coord.column;
                }
            }
            let avg_col = sum_col / chunk.len() as i32;
            let launch_column = avg_col.clamp(0, (width - 1).max(0));

            let burst_row_range = (height / 3).max(1);
            let burst_row = (top + rng.gen_range(0, burst_row_range)).clamp(top, bottom);
            let burst_coord = Coord::new(launch_column, burst_row);

            let color = rng.choice_color(&FIREWORK_COLORS);
            let shell_start = shell_idx as i64 * STAGGER;
            let n = chunk.len().max(1);

            for (i, &id) in chunk.iter().enumerate() {
                let final_coord = match terminal.get_character(id) {
                    Some(character) => character.input_coord,
                    None => continue,
                };

                let jitter = rng.gen_range(-2, 3);
                let launch_start =
                    Coord::new((launch_column + jitter).clamp(0, (width - 1).max(0)), bottom);

                let angle = (i as f64 / n as f64) * std::f64::consts::TAU;
                let radius = 3.0 + rng.gen_f64() * 5.0;
                let peak_col = (burst_coord.column as f64 + angle.cos() * radius).round() as i32;
                let peak_row =
                    (burst_coord.row as f64 + angle.sin() * radius * 0.5).round() as i32;
                let peak_coord = Coord::new(
                    peak_col.clamp(0, (width - 1).max(0)),
                    peak_row.clamp(top, bottom),
                );

                let launch_speed = 1.2;
                let burst_speed = 0.6;
                let fall_speed = 0.4;

                let launch_dist = distance(launch_start, burst_coord);
                let burst_dist = distance(burst_coord, peak_coord);
                let fall_dist = distance(peak_coord, final_coord);

                let launch_steps = ((launch_dist / launch_speed).ceil() as i64).max(1);
                let burst_steps = ((burst_dist / burst_speed).ceil() as i64).max(1);
                let fall_steps = ((fall_dist / fall_speed).ceil() as i64).max(1);

                plans.push(CharPlan {
                    id,
                    launch_start,
                    burst_coord,
                    peak_coord,
                    final_coord,
                    color,
                    shell_start,
                    launch_steps,
                    burst_steps,
                    fall_steps,
                });
            }
        }

        // Pre-build motion paths and animation scenes for every planned character.
        for plan in &plans {
            let launch_speed = 1.2;
            let burst_speed = 0.6;
            let fall_speed = 0.4;

            let launch_path = build_path(
                "launch",
                launch_speed,
                easing::ease_out_cubic,
                plan.launch_start,
                plan.burst_coord,
            );
            let burst_path = build_path(
                "burst",
                burst_speed,
                easing::ease_out_expo,
                plan.burst_coord,
                plan.peak_coord,
            );
            let fall_path = build_path(
                "fall",
                fall_speed,
                easing::ease_in_quad,
                plan.peak_coord,
                plan.final_coord,
            );

            if let Some(character) = terminal.get_character_mut(plan.id) {
                let input_symbol = character.input_symbol;

                character.motion.add_path(launch_path);
                character.motion.add_path(burst_path);
                character.motion.add_path(fall_path);

                let launch_scene = single_frame_scene("launch_scene", '.', plan.color);
                let burst_scene = single_frame_scene("burst_scene", '*', plan.color);
                let fall_scene =
                    single_frame_scene("fall_scene", input_symbol, dim(plan.color));

                character.animation.add_scene(launch_scene);
                character.animation.add_scene(burst_scene);
                character.animation.add_scene(fall_scene);

                character.set_visibility(false);
            }
        }

        let total_frames = plans
            .iter()
            .map(|plan| plan.shell_start + plan.launch_steps + plan.burst_steps + plan.fall_steps)
            .max()
            .unwrap_or(0)
            + 5;

        let mut frames_out: Vec<String> = Vec::with_capacity(total_frames.max(0) as usize);

        for t in 0..total_frames {
            for plan in &plans {
                let launch_end = plan.shell_start + plan.launch_steps;
                let burst_end = launch_end + plan.burst_steps;
                let fall_end = burst_end + plan.fall_steps;

                if t == plan.shell_start {
                    if let Some(character) = terminal.get_character_mut(plan.id) {
                        character.set_visibility(true);
                        character.motion.current_coord = plan.launch_start;
                        character.motion.current_pos =
                            (plan.launch_start.column as f64, plan.launch_start.row as f64);
                        character.motion.activate_path("launch");
                        character.animation.activate_scene("launch_scene");
                    }
                } else if t == launch_end {
                    if let Some(character) = terminal.get_character_mut(plan.id) {
                        character.motion.activate_path("burst");
                        character.animation.activate_scene("burst_scene");
                    }
                } else if t == burst_end {
                    if let Some(character) = terminal.get_character_mut(plan.id) {
                        character.motion.activate_path("fall");
                        character.animation.activate_scene("fall_scene");
                    }
                } else if t == fall_end {
                    if let Some(character) = terminal.get_character_mut(plan.id) {
                        character.motion.active_path_id = None;
                        character.motion.current_coord = plan.final_coord;
                        character.motion.current_pos =
                            (plan.final_coord.column as f64, plan.final_coord.row as f64);
                        character.animation.activate_scene("default");
                    }
                }
            }

            terminal.step_animation();
            frames_out.push(terminal.render());
        }

        frames_out
    }
}
