//! Errorcorrect effect: some character pairs start swapped, blink as "errors",
//! then wipe out, travel back to their correct positions along a correcting
//! color gradient, wipe back in and settle into the final gradient color.
//!
//! Port of terminaltexteffects/effects/effect_errorcorrect.py.

use std::time::{SystemTime, UNIX_EPOCH};

use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::character::EffectCharacter;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

const BLOCK_WIPE_START: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
const BLOCK_WIPE_END: [char; 7] = ['▇', '▆', '▅', '▄', '▃', '▂', '▁'];

/// Ratio of characters (as pairs) that start out swapped.
const ERROR_PAIRS: f64 = 0.1;
/// Ticks between activating successive swapped pairs.
const SWAP_DELAY: u32 = 10;
/// Speed of the return-to-input-coord movement.
const MOVEMENT_SPEED: f64 = 0.5;

/// Small xorshift PRNG so we do not depend on external crates.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn gen_range(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    /// Swapped, sitting at the wrong coordinate, not yet activated.
    Waiting,
    /// Blinking the error colors.
    Error,
    /// Wiping out with the block characters.
    WipeStart,
    /// Travelling back to the input coordinate on the correcting gradient.
    Moving,
    /// Wiping back in with the block characters.
    WipeEnd,
    /// Fading from the correct color to the final gradient color.
    FinalFade,
    /// Finished (also used for characters that were never swapped).
    Done,
}

pub struct Errorcorrect;

impl Errorcorrect {
    pub fn new() -> Self {
        Errorcorrect
    }
}

impl Default for Errorcorrect {
    fn default() -> Self {
        Errorcorrect::new()
    }
}

/// Build the paths and scenes for one swapped character, mirroring the
/// Python `_configure_swapped_character`.
fn configure_swapped_character(
    ch: &mut EffectCharacter,
    start_coord: Coord,
    error_color: Color,
    correct_color: Color,
    final_color: Color,
) {
    let input_symbol = ch.input_symbol;
    let input_coord = ch.input_coord;
    let white = Color::new(255, 255, 255);
    let black = Color::new(0, 0, 0);

    // Start at the other character's input coordinate.
    ch.motion.current_coord = start_coord;

    // Path back to this character's own input coordinate. The engine treats
    // the first waypoint as the origin, so add the start coord explicitly.
    let max_steps;
    {
        let path = ch.motion.new_path("input_coord", MOVEMENT_SPEED, None);
        path.add_waypoint(start_coord);
        path.add_waypoint(input_coord);
        max_steps = path.max_steps;
    }

    // Error blink: white-on-error / black-on-white, three cycles.
    {
        let scn = ch.animation.new_scene("error", false);
        for _ in 0..3 {
            scn.add_frame(
                input_symbol,
                3,
                ColorPair::new(Some(white), Some(error_color)),
                false,
            );
            scn.add_frame(
                input_symbol,
                3,
                ColorPair::new(Some(black), Some(white)),
                false,
            );
        }
    }

    // Block wipe out, in the error color.
    {
        let scn = ch.animation.new_scene("first_block_wipe", false);
        for block in BLOCK_WIPE_START {
            scn.add_frame(block, 3, ColorPair::fg(error_color), false);
        }
    }

    // Correcting gradient shown while travelling; frame durations are sized
    // so the gradient spans the movement (approximating DISTANCE sync).
    {
        let correcting_gradient = Gradient::new(&[error_color, correct_color], 10);
        let n_colors = correcting_gradient.spectrum.len().max(1);
        let duration = ((max_steps as usize + n_colors - 1) / n_colors).max(1) as u32;
        let scn = ch.animation.new_scene("correcting", false);
        for color in &correcting_gradient.spectrum {
            scn.add_frame('█', duration, ColorPair::fg(*color), false);
        }
    }

    // Block wipe back in, in the correct color.
    {
        let scn = ch.animation.new_scene("last_block_wipe", false);
        for block in BLOCK_WIPE_END {
            scn.add_frame(block, 3, ColorPair::fg(correct_color), false);
        }
    }

    // Fade from the correct color into the character's final gradient color.
    {
        let final_gradient = Gradient::new(&[correct_color, final_color], 10);
        let scn = ch.animation.new_scene("final", false);
        for color in &final_gradient.spectrum {
            scn.add_frame(input_symbol, 3, ColorPair::fg(*color), false);
        }
    }
}

impl Effect for Errorcorrect {
    fn name(&self) -> &str {
        "errorcorrect"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let height = terminal.canvas.height;
        let n = terminal.characters.len();

        let error_color = Color::from_hex("e74c3c").unwrap_or(Color::new(231, 76, 60));
        let correct_color = Color::from_hex("45bf55").unwrap_or(Color::new(69, 191, 85));
        let final_stops = [
            Color::from_hex("8A008A").unwrap_or(Color::new(138, 0, 138)),
            Color::from_hex("00D1FF").unwrap_or(Color::new(0, 209, 255)),
            Color::from_hex("FFFFFF").unwrap_or(Color::new(255, 255, 255)),
        ];
        let final_gradient = Gradient::new(&final_stops, 12);

        // Final color per character, mapped vertically (row 1 = bottom).
        let final_colors: Vec<Color> = terminal
            .characters
            .iter()
            .map(|c| {
                let fraction = if height > 1 {
                    (c.input_coord.row - 1) as f64 / (height - 1) as f64
                } else {
                    0.0
                };
                final_gradient
                    .get_color_at_fraction(fraction)
                    .unwrap_or(Color::new(255, 255, 255))
            })
            .collect();

        // Everyone visible, showing the final gradient color from the start.
        for (i, ch) in terminal.characters.iter_mut().enumerate() {
            ch.is_visible = true;
            let symbol = ch.input_symbol;
            ch.animation.current_visual =
                CharacterVisual::new(symbol, false, ColorPair::fg(final_colors[i]));
        }

        // Seed from wall clock mixed with an FNV hash of the input.
        let time_seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x1234_5678_9ABC_DEF0);
        let input_hash = input.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
            (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
        });
        let mut rng = Rng::new(time_seed ^ input_hash);

        // Pick the swapped pairs.
        let num_swaps = if n >= 2 {
            ((n as f64 * ERROR_PAIRS) as usize).max(1)
        } else {
            0
        };
        let mut pool: Vec<usize> = (0..n).collect();
        let mut swapped: Vec<(usize, usize)> = Vec::new();
        while swapped.len() < num_swaps && pool.len() >= 2 {
            let a = pool.remove(rng.gen_range(pool.len()));
            let b = pool.remove(rng.gen_range(pool.len()));
            swapped.push((a, b));
        }

        let mut states: Vec<State> = vec![State::Done; n];
        for &(a, b) in &swapped {
            let coord_a = terminal.characters[a].input_coord;
            let coord_b = terminal.characters[b].input_coord;
            configure_swapped_character(
                &mut terminal.characters[a],
                coord_b,
                error_color,
                correct_color,
                final_colors[a],
            );
            configure_swapped_character(
                &mut terminal.characters[b],
                coord_a,
                error_color,
                correct_color,
                final_colors[b],
            );
            states[a] = State::Waiting;
            states[b] = State::Waiting;
        }

        let mut pending: Vec<(usize, usize)> = swapped;
        let mut swap_delay: u32 = 0;

        let mut frames_out: Vec<String> = vec![terminal.get_formatted_output_string()];

        let mut guard: u32 = 0;
        loop {
            guard += 1;
            if guard > 100_000 {
                break;
            }

            // Activate the next swapped pair on the swap-delay cadence.
            if swap_delay == 0 {
                if !pending.is_empty() {
                    let (a, b) = pending.remove(0);
                    for &i in &[a, b] {
                        terminal.characters[i].animation.activate_scene("error");
                        states[i] = State::Error;
                    }
                    swap_delay = SWAP_DELAY;
                }
            } else {
                swap_delay -= 1;
            }

            // Tick active characters and run the per-character state machine.
            let mut all_done = pending.is_empty();
            for i in 0..n {
                match states[i] {
                    State::Done | State::Waiting => {}
                    _ => {
                        let ch = &mut terminal.characters[i];
                        ch.tick();
                        match states[i] {
                            State::Error => {
                                if ch.animation.active_scene_is_complete() {
                                    ch.animation.activate_scene("first_block_wipe");
                                    states[i] = State::WipeStart;
                                }
                            }
                            State::WipeStart => {
                                if ch.animation.active_scene_is_complete() {
                                    ch.animation.activate_scene("correcting");
                                    ch.motion.activate_path("input_coord");
                                    states[i] = State::Moving;
                                }
                            }
                            State::Moving => {
                                if ch.motion.movement_is_complete() {
                                    ch.animation.activate_scene("last_block_wipe");
                                    states[i] = State::WipeEnd;
                                }
                            }
                            State::WipeEnd => {
                                if ch.animation.active_scene_is_complete() {
                                    ch.animation.activate_scene("final");
                                    states[i] = State::FinalFade;
                                }
                            }
                            State::FinalFade => {
                                if ch.animation.active_scene_is_complete() {
                                    states[i] = State::Done;
                                }
                            }
                            State::Waiting | State::Done => {}
                        }
                    }
                }
                if states[i] != State::Done {
                    all_done = false;
                }
            }

            frames_out.push(terminal.get_formatted_output_string());

            if all_done && pending.is_empty() {
                break;
            }
        }

        frames_out
    }
}
