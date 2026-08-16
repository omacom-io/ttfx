//! Rings effect (port of terminaltexteffects/effects/effect_rings.py).
//!
//! Characters are shuffled and condensed into concentric rings around the
//! canvas center, spin around their rings, disperse and regroup a few times,
//! then return home colored with the final gradient. Characters that do not
//! fit into a ring simply hold their input position in the final gradient
//! color, matching the spirit of the Python original within this engine.

use std::collections::HashSet;
use std::f64::consts::PI;

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

const MAX_FRAMES: usize = 3000;
const SPIN_DISPERSE_CYCLES: usize = 3;
const RING_GAP_FRACTION: f64 = 0.1; // Python default `--ring-gap 0.1`
const CONDENSE_SPEED: f64 = 0.5;
const SPIN_SPEED: f64 = 0.25; // Python default `--spin-speed 0.25`
const DISPERSE_SPEED: f64 = 0.3; // Python disperse paths use ~0.14 looping; finite here
const HOME_SPEED: f64 = 0.8; // Python: home_path speed=0.8, ease=out_quad
const MAX_SPIN_ARC: usize = 30; // bounded arc per spin cycle (Python spins for a fixed duration)
const DISPERSE_JAUNTS: usize = 3;

/// Simple deterministic xorshift PRNG (stand-in for Python's `random`).
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.max(1))
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
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as i32
    }

    /// Fisher-Yates shuffle (Python: random.shuffle).
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

/// Port of geometry.find_coords_on_circle(origin, radius, 7 * radius, unique=True).
fn find_coords_on_circle(origin: Coord, radius: i32, coords_limit: usize) -> Vec<Coord> {
    let mut coords = Vec::new();
    let mut seen: HashSet<Coord> = HashSet::new();
    if coords_limit == 0 || radius <= 0 {
        return coords;
    }
    for i in 0..coords_limit {
        let angle = 2.0 * PI * (i as f64) / (coords_limit as f64);
        let column = (origin.column as f64 + radius as f64 * angle.cos()).round() as i32;
        let row = (origin.row as f64 + radius as f64 * angle.sin()).round() as i32;
        let coord = Coord::new(column, row);
        if seen.insert(coord) {
            coords.push(coord);
        }
    }
    coords
}

/// One concentric ring: its circle coords, color, spin direction and members.
struct Ring {
    coords: Vec<Coord>,
    color: Color,
    clockwise: bool,
    /// (character index in arena, current index into `coords`)
    members: Vec<(usize, usize)>,
}

fn run_phase(terminal: &mut Terminal, frames: &mut Vec<String>) {
    loop {
        if frames.len() >= MAX_FRAMES {
            break;
        }
        let active = terminal.tick();
        frames.push(terminal.get_formatted_output_string());
        if active == 0 {
            break;
        }
    }
}

fn final_color(gradient: &Gradient, coord: Coord, height: i32) -> Color {
    let fraction = if height > 1 {
        (coord.row - 1) as f64 / (height - 1) as f64
    } else {
        0.0
    };
    gradient
        .get_color_at_fraction(fraction)
        .unwrap_or(Color::new(255, 255, 255))
}

pub struct Rings;

impl Rings {
    pub fn new() -> Self {
        Rings
    }
}

impl Effect for Rings {
    fn name(&self) -> &str {
        "rings"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let mut frames: Vec<String> = Vec::new();
        if terminal.characters.is_empty() {
            return frames;
        }

        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;
        let center = terminal.canvas.center();
        let max_dim = width.max(height);
        // Python: ring_gap = max(round(max(right, top) * config.ring_gap), 1)
        let ring_gap = ((max_dim as f64 * RING_GAP_FRACTION).round() as i32).max(1);

        // Python defaults: --ring-colors ab48ff e7b2b2 fffebd
        let ring_colors = [
            Color::from_hex("ab48ff").unwrap(),
            Color::from_hex("e7b2b2").unwrap(),
            Color::from_hex("fffebd").unwrap(),
        ];
        // Python defaults: --final-gradient-stops 8A008A 00D1FF FFFFFF
        let final_gradient = Gradient::new(
            &[
                Color::from_hex("8A008A").unwrap(),
                Color::from_hex("00D1FF").unwrap(),
                Color::from_hex("FFFFFF").unwrap(),
            ],
            12,
        );

        let mut rng = Rng::new(0x9E37_79B9_7F4A_7C15);

        // Build the rings: radii 1, 1 + gap, ... while enough of the ring
        // lies within the canvas.
        let mut rings: Vec<Ring> = Vec::new();
        let mut radius = 1i32;
        while radius < max_dim {
            let coords = find_coords_on_circle(center, radius, (7 * radius) as usize);
            if coords.is_empty() {
                break;
            }
            let in_canvas = coords
                .iter()
                .filter(|c| terminal.canvas.coord_is_in_canvas(**c))
                .count();
            if (in_canvas as f64) / (coords.len() as f64) < 0.25 {
                break;
            }
            let color = ring_colors[rings.len() % ring_colors.len()];
            let clockwise = rings.len() % 2 == 0;
            rings.push(Ring {
                coords,
                color,
                clockwise,
                members: Vec::new(),
            });
            radius += ring_gap;
        }

        // Shuffle pending characters and fill the rings in order.
        let mut pending: Vec<usize> = (0..terminal.characters.len()).collect();
        rng.shuffle(&mut pending);
        let mut pending_iter = pending.into_iter();
        'assign: for ring in rings.iter_mut() {
            for slot in 0..ring.coords.len() {
                match pending_iter.next() {
                    Some(ci) => ring.members.push((ci, slot)),
                    None => break 'assign,
                }
            }
        }
        let leftover: Vec<usize> = pending_iter.collect();

        // Everyone is visible from the start (Python: set_character_visibility True).
        for character in terminal.get_characters_mut() {
            character.is_visible = true;
        }

        // --- Phase 1: condense into rings ------------------------------------
        for ring in &rings {
            for &(ci, slot) in &ring.members {
                let target = ring.coords[slot];
                let ch = &mut terminal.characters[ci];
                let scene = ch.animation.new_scene("ring", false);
                scene.add_frame(ch.input_symbol, 1, ColorPair::fg(ring.color), false);
                ch.animation.activate_scene("ring");
                let start = ch.input_coord;
                let path = ch
                    .motion
                    .new_path("condense", CONDENSE_SPEED, Some(easing::in_out_sine));
                path.add_waypoint(start);
                path.add_waypoint(target);
                ch.motion.activate_path("condense");
            }
        }
        for &ci in &leftover {
            let ch = &mut terminal.characters[ci];
            let color = final_color(&final_gradient, ch.input_coord, height);
            let scene = ch.animation.new_scene("start", false);
            scene.add_frame(ch.input_symbol, 1, ColorPair::fg(color), false);
            ch.animation.activate_scene("start");
        }
        run_phase(&mut terminal, &mut frames);

        // --- Phase 2: spin / disperse cycles ----------------------------------
        for cycle in 0..SPIN_DISPERSE_CYCLES {
            // Spin: traverse an arc of the ring; alternate direction per ring.
            let spin_id = format!("spin_{cycle}");
            for ring in rings.iter_mut() {
                let coords = &ring.coords;
                let clockwise = ring.clockwise;
                let len = coords.len();
                if len < 2 {
                    continue;
                }
                let arc = len.min(MAX_SPIN_ARC);
                for member in ring.members.iter_mut() {
                    let ci = member.0;
                    let mut idx = member.1;
                    let ch = &mut terminal.characters[ci];
                    let path = ch.motion.new_path(&spin_id, SPIN_SPEED, None);
                    path.add_waypoint(coords[idx]);
                    for _ in 0..arc {
                        idx = if clockwise {
                            (idx + len - 1) % len
                        } else {
                            (idx + 1) % len
                        };
                        path.add_waypoint(coords[idx]);
                    }
                    member.1 = idx;
                    ch.motion.activate_path(&spin_id);
                }
            }
            run_phase(&mut terminal, &mut frames);

            // Disperse: random jaunts near the ring point, then regroup.
            let disp_id = format!("disperse_{cycle}");
            for ring in rings.iter() {
                for &(ci, idx) in &ring.members {
                    let origin = ring.coords[idx];
                    let mut jaunts = Vec::with_capacity(DISPERSE_JAUNTS);
                    for _ in 0..DISPERSE_JAUNTS {
                        jaunts.push(Coord::new(
                            rng.gen_range(origin.column - ring_gap, origin.column + ring_gap),
                            rng.gen_range(origin.row - ring_gap, origin.row + ring_gap),
                        ));
                    }
                    let ch = &mut terminal.characters[ci];
                    let path = ch.motion.new_path(&disp_id, DISPERSE_SPEED, None);
                    path.add_waypoint(origin);
                    for coord in jaunts {
                        path.add_waypoint(coord);
                    }
                    path.add_waypoint(origin);
                    ch.motion.activate_path(&disp_id);
                }
            }
            run_phase(&mut terminal, &mut frames);
        }

        // --- Phase 3: return home in the final gradient ------------------------
        for ring in rings.iter() {
            for &(ci, idx) in &ring.members {
                let origin = ring.coords[idx];
                let ch = &mut terminal.characters[ci];
                let home = ch.input_coord;
                let path = ch
                    .motion
                    .new_path("home", HOME_SPEED, Some(easing::out_quad));
                path.add_waypoint(origin);
                path.add_waypoint(home);
                ch.motion.activate_path("home");
            }
        }
        for character in terminal.get_characters_mut() {
            let color = final_color(&final_gradient, character.input_coord, height);
            let scene = character.animation.new_scene("final", false);
            scene.add_frame(character.input_symbol, 1, ColorPair::fg(color), false);
            character.animation.activate_scene("final");
        }
        run_phase(&mut terminal, &mut frames);

        frames
    }
}
