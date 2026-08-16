//! Swarm effect: characters spawn off-canvas in swarms, drift between random
//! "swarm areas" while flashing, then settle onto their input coordinates.
//!
//! Port of terminaltexteffects/effects/effect_swarm.py, adapted to the
//! simplified engine in this crate (no event handlers: path chaining and
//! scene activation are driven from the frame loop).

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::geometry::{find_length_of_line, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

const BASE_COLOR: &str = "31a0d4";
const FLASH_COLOR: &str = "f2ea79";
const FINAL_GRADIENT_STOPS: [&str; 3] = ["8A008A", "00D1FF", "FFFFFF"];
const SWARM_SIZE_FRACTION: f64 = 0.1;
const MAX_FRAMES: usize = 10_000;

/// Minimal deterministic xorshift64 PRNG (the crate has no rand dependency).
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x5EED_5EED_5EED_5EED } else { seed })
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform float in [lo, hi).
    fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * ((self.next() >> 11) as f64 / (1u64 << 53) as f64)
    }

    /// Uniform integer in [lo, hi] (inclusive).
    fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next() % ((hi - lo + 1) as u64)) as i32
    }

    fn choice<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[(self.next() % items.len() as u64) as usize]
    }
}

/// All grid coords within `radius` of `center` (may extend past the canvas;
/// the renderer skips out-of-canvas coords).
fn coords_in_circle(center: Coord, radius: i32) -> Vec<Coord> {
    let mut out = Vec::new();
    for column in (center.column - radius)..=(center.column + radius) {
        for row in (center.row - radius)..=(center.row + radius) {
            let coord = Coord::new(column, row);
            if find_length_of_line(center, coord) <= radius as f64 {
                out.push(coord);
            }
        }
    }
    if out.is_empty() {
        out.push(center);
    }
    out
}

/// A random coordinate just outside the canvas (swarm spawn point).
fn random_outside_coord(rng: &mut Rng, width: i32, height: i32) -> Coord {
    match rng.range_i32(0, 3) {
        0 => Coord::new(-3, rng.range_i32(1, height)),
        1 => Coord::new(width + 3, rng.range_i32(1, height)),
        2 => Coord::new(rng.range_i32(1, width), -3),
        _ => Coord::new(rng.range_i32(1, width), height + 3),
    }
}

pub struct Swarm;

impl Swarm {
    pub fn new() -> Self {
        Swarm
    }
}

impl Default for Swarm {
    fn default() -> Self {
        Swarm::new()
    }
}

impl Effect for Swarm {
    fn name(&self) -> &str {
        "swarm"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let n = terminal.characters.len();
        if n == 0 {
            return vec![terminal.get_formatted_output_string()];
        }

        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;
        let mut rng = Rng::new(
            0x5EED ^ (input.len() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
        );

        // --- final gradient: horizontal mapping across the canvas ---
        let final_stops: Vec<Color> = FINAL_GRADIENT_STOPS
            .iter()
            .map(|h| Color::from_hex(h).expect("valid hex"))
            .collect();
        let final_gradient = Gradient::new(&final_stops, 12);
        let final_colors: Vec<Color> = terminal
            .characters
            .iter()
            .map(|c| {
                let fraction = if width > 1 {
                    (c.input_coord.column - 1) as f64 / (width - 1) as f64
                } else {
                    0.0
                };
                final_gradient
                    .get_color_at_fraction(fraction)
                    .expect("non-empty gradient")
            })
            .collect();

        let base_color = Color::from_hex(BASE_COLOR).expect("valid hex");
        let flash_color = Color::from_hex(FLASH_COLOR).expect("valid hex");

        // --- group characters into swarms (last swarm activates first,
        //     matching the Python `self.swarms.pop()` order) ---
        let swarm_size = ((n as f64 * SWARM_SIZE_FRACTION).round() as usize).max(1);
        let indices: Vec<usize> = (0..n).collect();
        let mut swarm_groups: Vec<Vec<usize>> =
            indices.chunks(swarm_size).map(|c| c.to_vec()).collect();
        swarm_groups.reverse();

        // Per-character queue of path ids to traverse in order.
        let mut plans: Vec<Vec<String>> = vec![Vec::new(); n];

        for group in &swarm_groups {
            // Mirrored base->flash gradient for this swarm's flash scene.
            let swarm_gradient = Gradient::new(&[base_color, flash_color], 7);
            let mut mirror: Vec<Color> = swarm_gradient.spectrum.clone();
            let mut back: Vec<Color> = swarm_gradient.spectrum.clone();
            back.reverse();
            mirror.extend(back);

            let spawn = random_outside_coord(&mut rng, width, height);

            // Random swarm areas within the inner 10%..90% of the canvas.
            let area_count = rng.range_i32(2, 4) as usize;
            let col_lo = ((width as f64 * 0.1).round() as i32).max(1);
            let col_hi = ((width as f64 * 0.9).round() as i32).max(col_lo);
            let row_lo = ((height as f64 * 0.1).round() as i32).max(1);
            let row_hi = ((height as f64 * 0.9).round() as i32).max(row_lo);
            let mut areas: Vec<Coord> = Vec::new();
            for _ in 0..100 {
                if areas.len() >= area_count {
                    break;
                }
                let coord = Coord::new(
                    rng.range_i32(col_lo, col_hi),
                    rng.range_i32(row_lo, row_hi),
                );
                if !areas.contains(&coord) {
                    areas.push(coord);
                }
            }
            if areas.is_empty() {
                areas.push(terminal.canvas.center());
            }

            let radius = ((width.min(height)) as f64 * 0.25).round().max(2.0) as i32;
            let area_coords: Vec<Vec<Coord>> =
                areas.iter().map(|a| coords_in_circle(*a, radius)).collect();

            for &ci in group {
                // Pick per-character targets inside each swarm area first so the
                // rng borrow does not overlap the character borrow.
                let targets: Vec<Coord> = area_coords
                    .iter()
                    .map(|coords| *rng.choice(coords))
                    .collect();
                let area_speeds: Vec<f64> = targets
                    .iter()
                    .map(|_| rng.uniform(0.5, 0.9))
                    .collect();

                let character = &mut terminal.characters[ci];
                let symbol = character.input_symbol;
                let input_coord = character.input_coord;
                character.motion.current_coord = spawn;

                // "flash" scene: base -> flash -> base; holds base when complete.
                {
                    let scene = character.animation.new_scene("flash", false);
                    for color in &mirror {
                        scene.add_frame(symbol, 2, ColorPair::fg(*color), false);
                    }
                }
                // "final" scene: one last flash ending on the final gradient color.
                {
                    let scene = character.animation.new_scene("final", false);
                    for color in &mirror {
                        scene.add_frame(symbol, 2, ColorPair::fg(*color), false);
                    }
                    scene.add_frame(symbol, 1, ColorPair::fg(final_colors[ci]), false);
                }

                // Chain paths: spawn -> area 0 -> area 1 -> ... -> input coord.
                // Each path starts where the previous one ends because this
                // engine has no automatic origin segment.
                let mut prev = spawn;
                let mut ids: Vec<String> = Vec::new();
                for (i, target) in targets.iter().enumerate() {
                    let id = i.to_string();
                    let path = character.motion.new_path(
                        &id,
                        area_speeds[i],
                        Some(easing::out_sine),
                    );
                    path.add_waypoint(prev);
                    path.add_waypoint(*target);
                    prev = *target;
                    ids.push(id);
                }
                let input_path =
                    character
                        .motion
                        .new_path("input_path", 0.6, Some(easing::in_out_quad));
                input_path.add_waypoint(prev);
                input_path.add_waypoint(input_coord);
                ids.push("input_path".to_string());

                plans[ci] = ids;
            }
        }

        // --- frame loop ---
        let mut cursor: Vec<usize> = vec![0; n];
        let mut activated: Vec<bool> = vec![false; n];
        let mut reached: Vec<bool> = vec![false; n];
        let mut active_swarm: Option<Vec<usize>> = None;
        let mut next_swarm = 0usize;

        let mut frames_out: Vec<String> = Vec::new();
        frames_out.push(terminal.get_formatted_output_string());

        loop {
            // Activate the next swarm once the current swarm's characters are
            // all on (or past) their input path — mirrors the Python gating.
            let ready = match &active_swarm {
                None => true,
                Some(group) => group
                    .iter()
                    .all(|&ci| reached[ci] || cursor[ci] + 1 >= plans[ci].len()),
            };
            if next_swarm < swarm_groups.len() && ready {
                let group = swarm_groups[next_swarm].clone();
                for &ci in &group {
                    let character = &mut terminal.characters[ci];
                    character.is_visible = true;
                    character.animation.activate_scene("flash");
                    let first = plans[ci][0].clone();
                    character.motion.activate_path(&first);
                    activated[ci] = true;
                }
                active_swarm = Some(group);
                next_swarm += 1;
            }

            // Tick every activated character; chain paths on completion.
            for ci in 0..n {
                if !activated[ci] {
                    continue;
                }
                let character = &mut terminal.characters[ci];
                character.tick();
                if character.motion.movement_is_complete() && !reached[ci] {
                    if cursor[ci] + 1 < plans[ci].len() {
                        cursor[ci] += 1;
                        let next_id = plans[ci][cursor[ci]].clone();
                        character.motion.activate_path(&next_id);
                        if next_id != "input_path" {
                            // Flash on arrival at each swarm area.
                            character.animation.activate_scene("flash");
                        }
                    } else {
                        reached[ci] = true;
                        character.animation.activate_scene("final");
                    }
                }
            }

            frames_out.push(terminal.get_formatted_output_string());

            let all_done = next_swarm >= swarm_groups.len()
                && (0..n).all(|ci| {
                    reached[ci]
                        && terminal.characters[ci]
                            .animation
                            .query_scene("final")
                            .map(|s| s.complete)
                            .unwrap_or(true)
                });
            if all_done || frames_out.len() >= MAX_FRAMES {
                break;
            }
        }

        frames_out
    }
}
