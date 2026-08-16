//! Blackhole effect: characters are scattered into a starfield, consumed by a
//! rotating black hole, collapsed into a singularity, and then explode back
//! to their input coordinates while cooling to a final gradient.
//!
//! Port of terminaltexteffects/effects/effect_blackhole.py, adapted to the
//! engine available in this crate (no event handlers, layers, or hold times:
//! the phases are orchestrated directly in `frames`).

use std::f64::consts::PI;

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

const MAX_FRAMES: usize = 20_000;

/// Minimal deterministic PRNG (splitmix-style LCG) so the effect needs no
/// external crates. Mirrors the Python effect's use of `random` in spirit.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }

    fn next_f64(&mut self) -> f64 {
        self.next_u32() as f64 / u32::MAX as f64
    }

    /// Inclusive integer range, like Python's random.randint.
    fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u32() % ((hi - lo + 1) as u32)) as i32
    }

    /// Float range, like Python's random.uniform.
    fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_f64()
    }

    fn choice<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[(self.next_u32() as usize) % items.len()]
    }

    /// Fisher-Yates, like Python's random.shuffle.
    fn shuffle<T>(&mut self, items: &mut [T]) {
        if items.len() < 2 {
            return;
        }
        for i in (1..items.len()).rev() {
            let j = (self.next_u32() as usize) % (i + 1);
            items.swap(i, j);
        }
    }
}

fn hex(s: &str) -> Color {
    Color::from_hex(s).expect("valid hex literal")
}

/// Points on a circle around `center`. Rows are compressed by 0.5 to
/// approximate the Python engine's double-row-diff so the ring looks round
/// in a terminal cell grid.
fn ring_positions(center: Coord, radius: f64, count: usize) -> Vec<Coord> {
    (0..count)
        .map(|i| {
            let theta = 2.0 * PI * i as f64 / count as f64;
            Coord::new(
                center.column + (radius * theta.cos()).round() as i32,
                center.row + (radius * theta.sin() * 0.5).round() as i32,
            )
        })
        .collect()
}

pub struct Blackhole;

impl Blackhole {
    pub fn new() -> Self {
        Blackhole
    }
}

impl Effect for Blackhole {
    fn name(&self) -> &str {
        "blackhole"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let mut out: Vec<String> = Vec::new();

        let n = terminal.characters.len();
        if n == 0 {
            return out;
        }

        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;
        let center = terminal.canvas.center();

        let mut rng = Rng::new(0x00b1_ac40_13d5_ee0d ^ (input.len() as u64));

        // ---- palette (defaults from the Python effect config) ----
        let blackhole_color = hex("ffffff");
        let star_colors = [
            hex("ffcc00"),
            hex("ffff66"),
            hex("ff9900"),
            hex("00ccff"),
            hex("ffffff"),
        ];
        let star_symbols = ['*', '.', '+', '·', '✸', '✶'];
        let flare_colors = [hex("ffcc00"), hex("ffff66"), hex("ffffff"), hex("ffff99")];
        let final_gradient = Gradient::new(&[hex("8A008A"), hex("00D1FF"), hex("ffffff")], 12);

        // ---- choose the black hole characters and ring geometry ----
        let mut order: Vec<usize> = (0..n).collect();
        rng.shuffle(&mut order);
        let bh_count = (n / 10).max(3).min(n);
        let blackhole: Vec<usize> = order[..bh_count].to_vec();
        let starfield: Vec<usize> = order[bh_count..].to_vec();

        let radius = ((width.min(height * 2) as f64) / 4.0).max(2.0);
        let ring = ring_positions(center, radius, bh_count);

        // ---- setup: black hole chars travel to the ring, others twinkle ----
        for (pos_idx, &i) in blackhole.iter().enumerate() {
            let ch = &mut terminal.characters[i];
            let scn = ch.animation.new_scene("blackhole", false);
            scn.add_frame('*', 1, ColorPair::fg(blackhole_color), false);
            ch.animation.activate_scene("blackhole");
            let cur = ch.motion.current_coord;
            let path = ch.motion.new_path("blackhole", 0.7, Some(easing::in_out_sine));
            path.add_waypoint(cur);
            path.add_waypoint(ring[pos_idx]);
            ch.motion.activate_path("blackhole");
            ch.is_visible = true;
        }
        for &i in &starfield {
            let ch = &mut terminal.characters[i];
            ch.motion.current_coord =
                Coord::new(rng.range_i32(1, width), rng.range_i32(1, height));
            let symbol = *rng.choice(&star_symbols);
            let color = *rng.choice(&star_colors);
            let scn = ch.animation.new_scene("star", false);
            scn.add_frame(symbol, 1, ColorPair::fg(color), false);
            ch.animation.activate_scene("star");
            ch.is_visible = true;
        }

        // ---- phase: form the black hole ring ----
        loop {
            terminal.tick();
            out.push(terminal.get_formatted_output_string());
            if out.len() >= MAX_FRAMES {
                return out;
            }
            let done = blackhole
                .iter()
                .all(|&i| terminal.characters[i].motion.movement_is_complete());
            if done {
                break;
            }
        }

        // ---- phase: rotate the ring while consuming the starfield ----
        for (pos_idx, &i) in blackhole.iter().enumerate() {
            let ch = &mut terminal.characters[i];
            let path = ch.motion.new_path("rotation", 0.45, None);
            for k in 0..=(bh_count * 2) {
                path.add_waypoint(ring[(pos_idx + k) % bh_count]);
            }
            ch.motion.activate_path("rotation");
        }

        let mut awaiting: Vec<usize> = starfield.clone();
        rng.shuffle(&mut awaiting);
        let mut consuming: Vec<usize> = Vec::new();
        let group_size = (starfield.len() / 15).max(1);
        let mut tick_count = 0usize;

        while !awaiting.is_empty() || !consuming.is_empty() {
            if tick_count % 3 == 0 {
                for _ in 0..group_size {
                    let Some(i) = awaiting.pop() else { break };
                    let ch = &mut terminal.characters[i];
                    let cur = ch.motion.current_coord;
                    let speed = rng.range_f64(0.17, 0.30);
                    let path = ch.motion.new_path("singularity", speed, Some(easing::in_expo));
                    path.add_waypoint(cur);
                    path.add_waypoint(center);
                    ch.motion.activate_path("singularity");
                    consuming.push(i);
                }
            }
            // keep the ring spinning
            for &i in &blackhole {
                if terminal.characters[i].motion.movement_is_complete() {
                    terminal.characters[i].motion.activate_path("rotation");
                }
            }

            terminal.tick();
            out.push(terminal.get_formatted_output_string());
            if out.len() >= MAX_FRAMES {
                return out;
            }

            // characters that reached the singularity are consumed
            let mut still_consuming = Vec::with_capacity(consuming.len());
            for i in consuming {
                if terminal.characters[i].motion.movement_is_complete() {
                    terminal.characters[i].is_visible = false;
                } else {
                    still_consuming.push(i);
                }
            }
            consuming = still_consuming;
            tick_count += 1;
        }

        // ---- phase: collapse the ring into a point ----
        for &i in &blackhole {
            let ch = &mut terminal.characters[i];
            let cur = ch.motion.current_coord;
            let path = ch.motion.new_path("collapse", 0.5, Some(easing::in_expo));
            path.add_waypoint(cur);
            path.add_waypoint(center);
            ch.motion.activate_path("collapse");
        }
        loop {
            terminal.tick();
            out.push(terminal.get_formatted_output_string());
            if out.len() >= MAX_FRAMES {
                return out;
            }
            let done = blackhole
                .iter()
                .all(|&i| terminal.characters[i].motion.movement_is_complete());
            if done {
                break;
            }
        }

        // brief singularity flare
        for &i in &blackhole {
            let ch = &mut terminal.characters[i];
            let scn = ch.animation.new_scene("point", false);
            for color in &flare_colors {
                scn.add_frame('*', 3, ColorPair::fg(*color), false);
            }
            ch.animation.activate_scene("point");
        }
        for _ in 0..12 {
            terminal.tick();
            out.push(terminal.get_formatted_output_string());
            if out.len() >= MAX_FRAMES {
                return out;
            }
        }

        // ---- phase: explosion back home, cooling to the final gradient ----
        let white = hex("ffffff");
        for i in 0..n {
            let star_color = *rng.choice(&star_colors);
            let angle = rng.range_f64(0.0, 2.0 * PI);
            let burst_radius = radius + rng.range_f64(2.0, 6.0);
            let speed = rng.range_f64(0.35, 0.7);

            let ch = &mut terminal.characters[i];
            let input_coord = ch.input_coord;
            let symbol = ch.input_symbol;

            let fraction = if height > 1 {
                (input_coord.row - 1) as f64 / (height - 1) as f64
            } else {
                1.0
            };
            let final_color = final_gradient.get_color_at_fraction(fraction).unwrap_or(white);

            let cooling = Gradient::new(&[white, star_color, final_color], 5);
            let scn = ch.animation.new_scene("cooling", false);
            for color in &cooling.spectrum {
                scn.add_frame(symbol, 3, ColorPair::fg(*color), false);
            }
            ch.animation.activate_scene("cooling");

            ch.motion.current_coord = center;
            let nearby = Coord::new(
                center.column + (burst_radius * angle.cos()).round() as i32,
                center.row + (burst_radius * angle.sin() * 0.5).round() as i32,
            );
            let path = ch.motion.new_path("home", speed, Some(easing::out_expo));
            path.add_waypoint(center);
            path.add_waypoint(nearby);
            path.add_waypoint(input_coord);
            ch.motion.activate_path("home");
            ch.is_visible = true;
        }

        loop {
            let active = terminal.tick();
            out.push(terminal.get_formatted_output_string());
            if active == 0 || out.len() >= MAX_FRAMES {
                break;
            }
        }

        // a few settle frames on the fully restored text
        for _ in 0..3 {
            terminal.tick();
            out.push(terminal.get_formatted_output_string());
            if out.len() >= MAX_FRAMES {
                break;
            }
        }

        out
    }
}
