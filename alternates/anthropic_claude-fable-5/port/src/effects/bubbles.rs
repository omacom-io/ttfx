//! Bubbles effect: characters are grouped into bubbles that float down the
//! canvas, pop when they reach the floor, and then fly to their input coords.
//!
//! Port of terminaltexteffects/effects/effect_bubbles.py adapted to the
//! simplified engine: bubble anchors are driven by standalone `Path`s and the
//! member characters are positioned on a circle around the anchor each frame.

use std::f64::consts::PI;

use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::motion::Path;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

const BUBBLE_SPEED: f64 = 0.1;
const POP_OUT_SPEED: f64 = 0.3;
const MOVEMENT_SPEED: f64 = 0.3;
const LAUNCH_INTERVAL: usize = 12;
const MAX_FRAMES: usize = 5000;

/// Minimal deterministic PRNG (xorshift64) so the effect needs no external crates.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Inclusive range.
    fn gen_range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i32
    }

    fn shuffle<T>(&mut self, v: &mut [T]) {
        if v.len() < 2 {
            return;
        }
        for i in (1..v.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            v.swap(i, j);
        }
    }
}

/// One bubble: an anchor path plus the character indices riding on its rim.
struct Bubble {
    members: Vec<usize>,
    path: Path,
    anchor: Coord,
    radius: i32,
    color: Color,
    popped: bool,
}

/// Character lifecycle stages after the bubble pops.
const STAGE_FLOATING: u8 = 0;
const STAGE_POPPING: u8 = 1;
const STAGE_FINAL: u8 = 2;

pub struct Bubbles;

impl Bubbles {
    pub fn new() -> Self {
        Bubbles
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
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;
        let char_count = terminal.characters.len();

        if char_count == 0 {
            return vec![terminal.get_formatted_output_string()];
        }

        let seed = input
            .bytes()
            .fold(0x9E37_79B9_7F4A_7C15u64, |acc, b| {
                acc.wrapping_mul(31).wrapping_add(b as u64)
            });
        let mut rng = Rng::new(seed);

        // Colors matching the Python defaults.
        let bubble_colors: Vec<Color> = ["d33aff", "7395c4", "43c2a7", "02ff7f"]
            .iter()
            .filter_map(|h| Color::from_hex(h))
            .collect();
        let pop_color = Color::from_hex("ffffff").unwrap_or(Color::new(255, 255, 255));
        let grad_start = Color::from_hex("d33aff").unwrap_or(Color::new(211, 58, 255));
        let grad_end = Color::from_hex("02ff7f").unwrap_or(Color::new(2, 255, 127));
        let final_gradient = Gradient::new(&[grad_start, grad_end], 24);

        // Diagonal final-gradient color per character.
        let denom = (((width - 1) + (height - 1)).max(1)) as f64;
        let final_colors: Vec<Color> = terminal
            .get_characters()
            .iter()
            .map(|ch| {
                let frac =
                    ((ch.input_coord.column - 1) + (ch.input_coord.row - 1)) as f64 / denom;
                final_gradient.get_color_at_fraction(frac).unwrap_or(grad_end)
            })
            .collect();

        // Group shuffled characters into bubbles.
        let radius = (width.min(height) / 6).max(1);
        let group_size = ((2.0 * PI * radius as f64).round() as usize).max(1);

        let mut order: Vec<usize> = (0..char_count).collect();
        rng.shuffle(&mut order);

        let mut bubbles: Vec<Bubble> = Vec::new();
        for chunk in order.chunks(group_size) {
            let start = Coord::new(rng.gen_range(1, width), height + radius);
            let floor = Coord::new(rng.gen_range(1, width), (1 + radius).min(height));
            let mut path = Path::new("float", BUBBLE_SPEED, None);
            path.add_waypoint(start);
            path.add_waypoint(floor);
            let color = if bubble_colors.is_empty() {
                pop_color
            } else {
                bubble_colors[(rng.next_u64() as usize) % bubble_colors.len()]
            };
            bubbles.push(Bubble {
                members: chunk.to_vec(),
                path,
                anchor: start,
                radius,
                color,
                popped: false,
            });
        }

        let mut stage: Vec<u8> = vec![STAGE_FLOATING; char_count];
        let mut frames_out: Vec<String> = Vec::new();
        let mut launched_count = 0usize;
        let mut frame_idx = 0usize;

        loop {
            // Launch bubbles on a fixed cadence.
            while launched_count < bubbles.len()
                && frame_idx >= launched_count * LAUNCH_INTERVAL
            {
                for &id in &bubbles[launched_count].members {
                    terminal.characters[id].is_visible = true;
                }
                launched_count += 1;
            }

            // Advance floating bubbles and position their members.
            for bubble in bubbles.iter_mut().take(launched_count) {
                if bubble.popped {
                    continue;
                }
                if let Some(coord) = bubble.path.step() {
                    bubble.anchor = coord;
                }
                let n = bubble.members.len().max(1);
                for (k, &id) in bubble.members.iter().enumerate() {
                    let angle = 2.0 * PI * (k as f64) / (n as f64);
                    let dc = (bubble.radius as f64 * angle.cos()).round() as i32;
                    let dr = (bubble.radius as f64 * angle.sin()).round() as i32;
                    let ch = &mut terminal.characters[id];
                    ch.motion.current_coord =
                        Coord::new(bubble.anchor.column + dc, bubble.anchor.row + dr);
                    ch.animation.current_visual = CharacterVisual::new(
                        ch.input_symbol,
                        false,
                        ColorPair::fg(bubble.color),
                    );
                }

                // Pop the bubble when it reaches the floor.
                if bubble.path.is_complete() {
                    bubble.popped = true;
                    let pop_radius = (bubble.radius + 3) as f64;
                    for (k, &id) in bubble.members.iter().enumerate() {
                        let angle = 2.0 * PI * (k as f64) / (n as f64);
                        let pop_coord = Coord::new(
                            bubble.anchor.column + (pop_radius * angle.cos()).round() as i32,
                            bubble.anchor.row + (pop_radius * angle.sin()).round() as i32,
                        );
                        let final_color = final_colors[id];
                        let ch = &mut terminal.characters[id];
                        let start_coord = ch.motion.current_coord;
                        let input_coord = ch.input_coord;
                        let input_symbol = ch.input_symbol;

                        let pop_path =
                            ch.motion.new_path("pop_out", POP_OUT_SPEED, Some(easing::out_expo));
                        pop_path.add_waypoint(start_coord);
                        pop_path.add_waypoint(pop_coord);

                        let final_path = ch.motion.new_path(
                            "final",
                            MOVEMENT_SPEED,
                            Some(easing::in_out_sine),
                        );
                        final_path.add_waypoint(pop_coord);
                        final_path.add_waypoint(input_coord);

                        ch.motion.activate_path("pop_out");

                        let scn = ch.animation.new_scene("pop", false);
                        scn.add_frame('*', 3, ColorPair::fg(pop_color), false);
                        scn.add_frame('\'', 3, ColorPair::fg(pop_color), false);
                        scn.add_frame(input_symbol, 1, ColorPair::fg(final_color), false);
                        ch.animation.activate_scene("pop");

                        stage[id] = STAGE_POPPING;
                    }
                }
            }

            // Tick popped characters through pop-out and homing motion.
            for id in 0..char_count {
                match stage[id] {
                    STAGE_POPPING => {
                        let ch = &mut terminal.characters[id];
                        ch.tick();
                        if ch.motion.movement_is_complete() {
                            ch.motion.activate_path("final");
                            stage[id] = STAGE_FINAL;
                        }
                    }
                    STAGE_FINAL => {
                        terminal.characters[id].tick();
                    }
                    _ => {}
                }
            }

            frames_out.push(terminal.get_formatted_output_string());
            frame_idx += 1;

            let all_done = launched_count == bubbles.len()
                && bubbles.iter().all(|b| b.popped)
                && (0..char_count).all(|id| {
                    stage[id] == STAGE_FINAL
                        && terminal.characters[id].motion.movement_is_complete()
                        && terminal.characters[id].animation.active_scene_is_complete()
                });
            if all_done || frames_out.len() >= MAX_FRAMES {
                break;
            }
        }

        frames_out
    }
}
