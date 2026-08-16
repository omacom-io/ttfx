//! Binary path effect: characters emerge from a scattered field of their own
//! Unicode binary representation and converge on their home coordinate,
//! resolving into the plain input text once they arrive.
//!
//! This is a best-effort port of `terminaltexteffects/effects/effect_binarypath.py`
//! constrained to the engine primitives actually available in this crate:
//! per-tick position is driven manually (via the public `Motion` fields)
//! rather than through `engine::motion::Path`, since `Motion::step` only
//! advances a path when `active_path_id` is set — we never set it, so
//! stepping the terminal's animation each tick is safe and leaves our
//! manual coordinate assignments untouched.

use super::Effect;

use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::terminal::Terminal;
use crate::utils::easing::{self, EasingFunction};
use crate::utils::geometry::{self, Coord};
use crate::utils::graphics::{Color, ColorPair};

pub struct Binarypath;

impl Binarypath {
    pub fn new() -> Self {
        Binarypath
    }
}

/// Minimal deterministic xorshift32 PRNG so the effect doesn't depend on an
/// external rng module (none exists in this crate yet).
fn next_rand(state: &mut u32) -> u32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}

fn make_visual(symbol: char, colors: Option<ColorPair>) -> CharacterVisual {
    let mut visual = CharacterVisual::new(symbol);
    visual.colors = colors;
    visual.formatted_symbol = visual.format_symbol();
    visual
}

const BINARY_PALETTE: [u8; 6] = [28, 29, 35, 36, 50, 51];
const EASE_CHOICES: [EasingFunction; 3] =
    [easing::ease_out_cubic, easing::ease_out_quad, easing::ease_in_out_sine];

struct CharState {
    id: u32,
    start: Coord,
    target: Coord,
    delay: u32,
    travel_ticks: u32,
    ease: EasingFunction,
    binary_frames: Vec<char>,
    color: Color,
    activated: bool,
    arrived: bool,
}

impl Effect for Binarypath {
    fn name(&self) -> &str {
        "binarypath"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let width = terminal.config.width.max(1) as i32;
        let height = terminal.config.height.max(1) as i32;

        let mut states: Vec<CharState> = Vec::new();
        {
            for character in terminal.get_characters() {
                if character.input_symbol == ' ' {
                    continue;
                }
                let mut seed = character
                    .id
                    .wrapping_mul(2_654_435_761)
                    .wrapping_add(0x9E37_79B9);
                let r1 = next_rand(&mut seed);
                let r2 = next_rand(&mut seed);
                let r3 = next_rand(&mut seed);
                let r4 = next_rand(&mut seed);

                let from_top = (r1 & 1) == 0;
                let start_row = if from_top {
                    -(((r2 % height as u32) + 1) as i32)
                } else {
                    height + ((r2 % height as u32) + 1) as i32
                };
                let start_col = (r3 % width as u32) as i32;
                let start = Coord::new(start_col, start_row);
                let target = character.input_coord;

                let speed = 1.0 + ((r4 % 4) as f64) * 0.4;
                let distance = geometry::distance(start, target);
                let travel_ticks = ((distance / speed).ceil() as u32).max(1);

                let binary_str = format!("{:b}", character.input_symbol as u32);
                let binary_frames: Vec<char> = binary_str.chars().collect();

                let delay = r1 % 12;
                let ease = EASE_CHOICES[(r3 as usize) % EASE_CHOICES.len()];
                let color = Color::Ansi256(BINARY_PALETTE[(r4 as usize) % BINARY_PALETTE.len()]);

                states.push(CharState {
                    id: character.id,
                    start,
                    target,
                    delay,
                    travel_ticks,
                    ease,
                    binary_frames,
                    color,
                    activated: false,
                    arrived: false,
                });
            }
        }

        if states.is_empty() {
            return vec![terminal.render()];
        }

        // Hide every animated character until its delay elapses.
        for state in &states {
            terminal.set_character_visibility(state.id, false);
        }

        let mut frames_out: Vec<String> = Vec::new();
        let max_ticks: u32 = 600;
        let hold_ticks: u32 = 20;
        let mut settle: u32 = 0;
        let mut tick: u32 = 0;

        loop {
            // Activate characters whose delay has elapsed.
            for state in states.iter_mut() {
                if !state.activated && tick >= state.delay {
                    state.activated = true;
                    if let Some(character) = terminal.get_character_mut(state.id) {
                        character.set_visibility(true);
                        character.motion.current_coord = state.start;
                        character.motion.current_pos =
                            (state.start.column as f64, state.start.row as f64);

                        let mut binary_scene = Scene::new("binary");
                        binary_scene.is_looping = true;
                        for &digit in &state.binary_frames {
                            binary_scene.add_frame(
                                make_visual(digit, Some(ColorPair::new(Some(state.color), None))),
                                2,
                            );
                        }
                        character.animation.add_scene(binary_scene);
                        character.animation.activate_scene("binary");
                    }
                }
            }

            // Manually advance position for activated, not-yet-arrived characters.
            for state in states.iter_mut() {
                if state.activated && !state.arrived {
                    let elapsed = tick.saturating_sub(state.delay) + 1;
                    let mut t = elapsed as f64 / state.travel_ticks as f64;
                    let just_arrived = t >= 1.0;
                    if just_arrived {
                        t = 1.0;
                    }
                    let eased_t = (state.ease)(t.clamp(0.0, 1.0));
                    let (x, y) = geometry::lerp(state.start, state.target, eased_t);
                    if let Some(character) = terminal.get_character_mut(state.id) {
                        character.motion.current_pos = (x, y);
                        character.motion.current_coord =
                            Coord::new(x.round() as i32, y.round() as i32);
                    }

                    if just_arrived {
                        state.arrived = true;
                        if let Some(character) = terminal.get_character_mut(state.id) {
                            character.motion.current_coord = state.target;
                            character.motion.current_pos =
                                (state.target.column as f64, state.target.row as f64);
                            let symbol = character.input_symbol;
                            let mut resolved = Scene::new("resolved");
                            resolved.add_frame(
                                make_visual(
                                    symbol,
                                    Some(ColorPair::new(Some(Color::Ansi256(15)), None)),
                                ),
                                3,
                            );
                            resolved.add_frame(make_visual(symbol, None), 1);
                            character.animation.add_scene(resolved);
                            character.animation.activate_scene("resolved");
                        }
                    }
                }
            }

            // Advance per-character animation scenes (motion.step is a no-op
            // here since we never set an active_path_id).
            terminal.step_animation();

            frames_out.push(terminal.render());
            tick += 1;

            let all_done = states.iter().all(|s| s.arrived);
            if all_done {
                settle += 1;
                if settle >= hold_ticks {
                    break;
                }
            }
            if tick >= max_ticks {
                break;
            }
        }

        frames_out
    }
}
