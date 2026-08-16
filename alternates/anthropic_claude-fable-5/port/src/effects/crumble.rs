//! Crumble: characters weaken into dust, crumble to the canvas floor, then the
//! dust is vacuumed up through the center of the canvas and each character is
//! restored to its input position with a strengthening flash.
//!
//! Port of terminaltexteffects/effects/effect_crumble.py, driven manually
//! (this engine has no event handlers), with phase logic in `frames()`.

use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Small deterministic xorshift PRNG so the effect needs no external crates.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x5CE1FF } else { seed })
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Inclusive range `lo..=hi`.
    fn gen_range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            return lo;
        }
        lo + (self.next() as usize) % (hi - lo + 1)
    }

    fn shuffle<T>(&mut self, v: &mut [T]) {
        if v.len() < 2 {
            return;
        }
        for i in (1..v.len()).rev() {
            let j = (self.next() as usize) % (i + 1);
            v.swap(i, j);
        }
    }
}

/// Per-character lifecycle stage, replacing the Python event-handler chains.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Waiting,
    Weakening,
    Falling,
    Fallen,
    Rising,
    Returning,
    Flashing,
    Strengthening,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Crumble,
    Vacuum,
    Complete,
}

pub struct Crumble;

impl Crumble {
    pub fn new() -> Self {
        Crumble
    }
}

impl Default for Crumble {
    fn default() -> Self {
        Crumble::new()
    }
}

impl Effect for Crumble {
    fn name(&self) -> &str {
        "crumble"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let mut rng = Rng::new(0x00C4_0B13_5CE1);

        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;
        let center = terminal.canvas.center();
        let bottom = 1i32;
        let top = height;

        let white = Color::new(255, 255, 255);
        // Upstream default final gradient stops: 5CE1FF -> FFFFFF, 12 steps, diagonal.
        let final_gradient = Gradient::new(
            &[Color::from_hex("5CE1FF").unwrap_or(white), white],
            12,
        );
        let dust_colors = [
            Color::from_hex("7d7d7d").unwrap_or(white),
            Color::from_hex("766b69").unwrap_or(white),
            Color::from_hex("848789").unwrap_or(white),
            Color::from_hex("9E9E8D").unwrap_or(white),
        ];

        let char_count = terminal.get_characters().len();
        let mut stages = vec![Stage::Waiting; char_count];

        // --- build: scenes and paths per character (Python __init__/build) ---
        {
            let diagonal_span = ((width - 1) + (height - 1)).max(1) as f64;
            for character in terminal.get_characters_mut() {
                let col = character.input_coord.column;
                let row = character.input_coord.row;
                let fraction =
                    (((col - 1) + (row - 1)) as f64 / diagonal_span).clamp(0.0, 1.0);
                let final_color = final_gradient
                    .get_color_at_fraction(fraction)
                    .unwrap_or(white);
                let dust_color = dust_colors[rng.gen_range(0, dust_colors.len() - 1)];

                character.is_visible = true;
                character.animation.current_visual = CharacterVisual::new(
                    character.input_symbol,
                    false,
                    ColorPair::fg(final_color),
                );

                // "weaken": fade from the final color down to dust.
                let weaken_gradient = Gradient::new(&[final_color, dust_color], 9);
                let symbol = character.input_symbol;
                {
                    let scene = character.animation.new_scene("weaken", false);
                    for color in &weaken_gradient.spectrum {
                        scene.add_frame(symbol, 4, ColorPair::fg(*color), false);
                    }
                }
                // "dust": resting dust visual while falling / lying on the floor.
                {
                    let scene = character.animation.new_scene("dust", false);
                    scene.add_frame(symbol, 1, ColorPair::fg(dust_color), false);
                }
                // "strengthen_flash": quick surge from dust to white.
                let flash_gradient = Gradient::new(&[dust_color, white], 6);
                {
                    let scene = character.animation.new_scene("strengthen_flash", false);
                    for color in &flash_gradient.spectrum {
                        scene.add_frame(symbol, 3, ColorPair::fg(*color), false);
                    }
                }
                // "strengthen": settle from white back to the final color.
                let strengthen_gradient = Gradient::new(&[white, final_color], 9);
                {
                    let scene = character.animation.new_scene("strengthen", false);
                    for color in &strengthen_gradient.spectrum {
                        scene.add_frame(symbol, 3, ColorPair::fg(*color), false);
                    }
                }

                // "fall": crumble to the canvas floor (Python: speed 0.2, out_bounce).
                {
                    let path = character.motion.new_path("fall", 0.2, Some(easing::out_cubic));
                    path.add_waypoint(Coord::new(col, row));
                    path.add_waypoint(Coord::new(col, bottom));
                }
                // "top": vacuumed up through the canvas center toward the top
                // (Python used a bezier control at the center; approximated with
                // an intermediate waypoint).
                {
                    let path = character.motion.new_path("top", 1.0, Some(easing::out_cubic));
                    path.add_waypoint(Coord::new(col, bottom));
                    path.add_waypoint(center);
                    path.add_waypoint(Coord::new(col, top));
                }
                // "input": drift back down into the original position.
                {
                    let path = character.motion.new_path("input", 0.3, None);
                    path.add_waypoint(Coord::new(col, top));
                    path.add_waypoint(Coord::new(col, row));
                }
            }
        }

        let mut pending: Vec<usize> = (0..char_count).collect();
        rng.shuffle(&mut pending);
        let mut vacuum_pending: Vec<usize> = Vec::new();

        let mut phase = if char_count == 0 {
            Phase::Complete
        } else {
            Phase::Crumble
        };

        let mut frames: Vec<String> = Vec::new();
        let mut frame_idx: u64 = 0;
        let max_frames = 20_000usize;

        for _ in 0..max_frames {
            // --- phase logic (activations) ---
            match phase {
                Phase::Crumble => {
                    if frame_idx % 2 == 0 && !pending.is_empty() {
                        let count = rng.gen_range(1, 3);
                        for _ in 0..count {
                            if let Some(idx) = pending.pop() {
                                let character = &mut terminal.get_characters_mut()[idx];
                                character.animation.activate_scene("weaken");
                                stages[idx] = Stage::Weakening;
                            }
                        }
                    }
                    if pending.is_empty() && stages.iter().all(|s| *s == Stage::Fallen) {
                        phase = Phase::Vacuum;
                        vacuum_pending = (0..char_count).collect();
                        rng.shuffle(&mut vacuum_pending);
                    }
                }
                Phase::Vacuum => {
                    if !vacuum_pending.is_empty() {
                        let count = rng.gen_range(1, 5);
                        for _ in 0..count {
                            if let Some(idx) = vacuum_pending.pop() {
                                let character = &mut terminal.get_characters_mut()[idx];
                                character.motion.activate_path("top");
                                stages[idx] = Stage::Rising;
                            }
                        }
                    }
                    if vacuum_pending.is_empty() && stages.iter().all(|s| *s == Stage::Done) {
                        phase = Phase::Complete;
                    }
                }
                Phase::Complete => {}
            }

            // --- advance the simulation one tick ---
            let active = terminal.tick();

            // --- per-character stage transitions (event-handler replacement) ---
            {
                let characters = terminal.get_characters_mut();
                for (idx, stage) in stages.iter_mut().enumerate() {
                    let character = &mut characters[idx];
                    match *stage {
                        Stage::Weakening => {
                            if character.animation.active_scene_is_complete() {
                                character.animation.activate_scene("dust");
                                character.motion.activate_path("fall");
                                *stage = Stage::Falling;
                            }
                        }
                        Stage::Falling => {
                            if character.motion.movement_is_complete() {
                                *stage = Stage::Fallen;
                            }
                        }
                        Stage::Rising => {
                            if character.motion.movement_is_complete() {
                                character.motion.activate_path("input");
                                *stage = Stage::Returning;
                            }
                        }
                        Stage::Returning => {
                            if character.motion.movement_is_complete() {
                                character.animation.activate_scene("strengthen_flash");
                                *stage = Stage::Flashing;
                            }
                        }
                        Stage::Flashing => {
                            if character.animation.active_scene_is_complete() {
                                character.animation.activate_scene("strengthen");
                                *stage = Stage::Strengthening;
                            }
                        }
                        Stage::Strengthening => {
                            if character.animation.active_scene_is_complete() {
                                *stage = Stage::Done;
                            }
                        }
                        Stage::Waiting | Stage::Fallen | Stage::Done => {}
                    }
                }
            }

            frames.push(terminal.get_formatted_output_string());
            frame_idx += 1;

            if phase == Phase::Complete && active == 0 {
                break;
            }
        }

        frames
    }
}
