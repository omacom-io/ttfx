//! Binarypath effect (port of terminaltexteffects/effects/effect_binarypath.py).
//!
//! Each input character is decomposed into its binary representation. The
//! binary digits spawn just outside the canvas and travel along right-angled
//! ("digital") paths to the character's input coordinate. Once every digit of
//! a group has arrived, the digits vanish and the source character collapses
//! into view, fading from a binary green to its final gradient color.

use std::collections::VecDeque;

use super::Effect;
use crate::engine::character::EffectCharacter;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Safety cap so a pathological input can never loop forever.
const MAX_FRAMES: usize = 20_000;

/// Fraction of binary groups that may be traveling at once (upstream default).
const ACTIVE_BINARY_GROUPS: f64 = 0.05;

/// Movement speed of the binary digits (upstream default).
const MOVEMENT_SPEED: f64 = 1.0;

/// Small deterministic xorshift PRNG (no external crates available).
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

    /// Inclusive range, mirroring Python's `random.randint(lo, hi)`.
    fn randint(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % (hi - lo + 1) as u64) as i32
    }

    fn index(&mut self, len: usize) -> usize {
        if len <= 1 {
            0
        } else {
            (self.next_u64() % len as u64) as usize
        }
    }
}

/// Mirrors the Python `BinaryRepresentation` bookkeeping (ids into the arena).
struct BinaryRep {
    source_id: usize,
    input_coord: Coord,
    bin_ids: Vec<usize>,
    pending: VecDeque<usize>,
}

pub struct Binarypath;

impl Binarypath {
    pub fn new() -> Self {
        Binarypath
    }

    /// Random coordinate one cell outside the canvas edge
    /// (stand-in for Python's `canvas.random_coord(outside_scope=True)`).
    fn random_coord_outside(rng: &mut Rng, width: i32, height: i32) -> Coord {
        match rng.index(4) {
            0 => Coord::new(0, rng.randint(1, height)),
            1 => Coord::new(width + 1, rng.randint(1, height)),
            2 => Coord::new(rng.randint(1, width), 0),
            _ => Coord::new(rng.randint(1, width), height + 1),
        }
    }

    /// Build the right-angled zig-zag path from `start` to `target`,
    /// matching the Python path-construction loop.
    fn build_path_coords(rng: &mut Rng, start: Coord, target: Coord) -> Vec<Coord> {
        let mut coords = vec![start];
        // last_orientation = random.choice(("col", "row"))
        let mut column_orientation = rng.index(2) == 0;
        loop {
            let last = *coords.last().expect("path never empty");
            if last == target {
                break;
            }
            let column_direction = (target.column - last.column).signum();
            let row_direction = (target.row - last.row).signum();
            let max_column_distance = (last.column - target.column).abs();
            let max_row_distance = (last.row - target.row).abs();
            let next = if column_orientation && max_row_distance > 0 {
                column_orientation = false;
                Coord::new(
                    last.column,
                    last.row + rng.randint(1, max_row_distance.min(10)) * row_direction,
                )
            } else if !column_orientation && max_column_distance > 0 {
                column_orientation = true;
                Coord::new(
                    last.column + rng.randint(1, max_column_distance.min(10)) * column_direction,
                    last.row,
                )
            } else {
                target
            };
            coords.push(next);
        }
        coords
    }
}

impl Effect for Binarypath {
    fn name(&self) -> &str {
        "binarypath"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;

        // Deterministic seed derived from the input.
        let seed = input
            .bytes()
            .fold(0x9E37_79B9_7F4A_7C15u64, |acc, b| {
                acc.wrapping_mul(31).wrapping_add(b as u64)
            });
        let mut rng = Rng::new(seed);

        // Upstream defaults.
        let binary_colors: Vec<Color> = ["044E29", "157e38", "45bf55", "95ed87"]
            .iter()
            .filter_map(|h| Color::from_hex(h))
            .collect();
        let stops = [
            Color::from_hex("00d500").expect("valid hex"),
            Color::from_hex("007500").expect("valid hex"),
        ];
        let final_gradient = Gradient::new(&stops, 12);

        // Snapshot the source characters (id, symbol, coord) before we start
        // appending binary characters to the arena.
        let sources: Vec<(usize, char, Coord)> = terminal
            .get_characters()
            .iter()
            .map(|c| (c.character_id, c.input_symbol, c.input_coord))
            .collect();

        // Per-source final color (vertical gradient across the canvas).
        let final_color_for = |coord: Coord| -> Color {
            let fraction = if height > 1 {
                (coord.row - 1) as f64 / (height - 1) as f64
            } else {
                0.0
            };
            final_gradient
                .get_color_at_fraction(fraction)
                .unwrap_or(stops[0])
        };

        // Build the collapse scene on every source character.
        for &(source_id, symbol, coord) in &sources {
            let start_color = binary_colors[rng.index(binary_colors.len())];
            let collapse_gradient = Gradient::new(&[start_color, final_color_for(coord)], 10);
            let character = &mut terminal.characters[source_id];
            let scene = character.animation.new_scene("collapse", false);
            for color in &collapse_gradient.spectrum {
                scene.add_frame(symbol, 5, ColorPair::fg(*color), false);
            }
        }

        // Build the binary representations and their travel paths.
        let mut pending_reps: Vec<BinaryRep> = Vec::new();
        for &(source_id, symbol, input_coord) in &sources {
            let start = Self::random_coord_outside(&mut rng, width, height);
            let path_coords = Self::build_path_coords(&mut rng, start, input_coord);

            let mut bin_ids = Vec::new();
            for bit in format!("{:b}", symbol as u32).chars() {
                let id = terminal.characters.len();
                let mut bin_char = EffectCharacter::new(id, bit, input_coord);
                bin_char.motion.current_coord = path_coords[0];
                {
                    let path = bin_char.motion.new_path("digital", MOVEMENT_SPEED, None);
                    for coord in &path_coords {
                        path.add_waypoint(*coord);
                    }
                }
                {
                    let color = binary_colors[rng.index(binary_colors.len())];
                    let scene = bin_char.animation.new_scene("color", false);
                    scene.add_frame(bit, 1, ColorPair::fg(color), false);
                }
                terminal.characters.push(bin_char);
                bin_ids.push(id);
            }
            let pending: VecDeque<usize> = bin_ids.iter().copied().collect();
            pending_reps.push(BinaryRep {
                source_id,
                input_coord,
                bin_ids,
                pending,
            });
        }

        let max_active_groups =
            (((pending_reps.len() as f64) * ACTIVE_BINARY_GROUPS) as usize).max(1);

        let mut active_reps: Vec<BinaryRep> = Vec::new();
        let mut collapsing_sources: Vec<usize> = Vec::new();
        let mut frames: Vec<String> = Vec::new();

        while (!pending_reps.is_empty() || !active_reps.is_empty() || !collapsing_sources.is_empty())
            && frames.len() < MAX_FRAMES
        {
            // Activate at most one new binary group per frame (as upstream).
            if !pending_reps.is_empty() && active_reps.len() < max_active_groups {
                let idx = rng.index(pending_reps.len());
                active_reps.push(pending_reps.swap_remove(idx));
            }

            // Launch one binary digit per active group per frame; when a group
            // has fully arrived, hide its digits and reveal the source.
            let mut finished: Vec<usize> = Vec::new();
            for (rep_index, rep) in active_reps.iter_mut().enumerate() {
                if let Some(next_id) = rep.pending.pop_front() {
                    let bin_char = &mut terminal.characters[next_id];
                    bin_char.is_visible = true;
                    bin_char.animation.activate_scene("color");
                    bin_char.motion.activate_path("digital");
                } else {
                    let travel_complete = rep.bin_ids.iter().all(|&id| {
                        let c = &terminal.characters[id];
                        c.motion.movement_is_complete()
                            && c.motion.current_coord == rep.input_coord
                    });
                    if travel_complete {
                        for &id in &rep.bin_ids {
                            terminal.characters[id].is_visible = false;
                        }
                        let source = &mut terminal.characters[rep.source_id];
                        source.is_visible = true;
                        source.animation.activate_scene("collapse");
                        collapsing_sources.push(rep.source_id);
                        finished.push(rep_index);
                    }
                }
            }
            for rep_index in finished.into_iter().rev() {
                active_reps.remove(rep_index);
            }

            terminal.tick();
            collapsing_sources
                .retain(|&id| !terminal.characters[id].animation.active_scene_is_complete());

            frames.push(terminal.get_formatted_output_string());
        }

        // Final resting frame with every character resolved.
        frames.push(terminal.get_formatted_output_string());
        frames
    }
}
