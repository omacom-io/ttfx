//! Laser etch: a laser beam sweeps across the text, etching each character
//! into place with a shower of cooling sparks.
//!
//! Port of TTE's `laseretch` effect adapted to this engine: the laser head
//! travels serpentine, row by row from the top of the text. As it passes a
//! character's input coordinate the character is revealed white-hot and
//! cools through a spark gradient down to its final gradient color, while a
//! few spark particles arc away from the etch point and fade out.

use std::collections::HashMap;

use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::character::EffectCharacter;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Cells the laser head advances per tick.
const ETCH_SPEED: usize = 2;
/// Hard safety cap on emitted frames.
const MAX_FRAMES: usize = 20_000;

/// Small deterministic xorshift RNG (no external rand dependency).
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Inclusive integer range.
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next() % ((hi - lo + 1) as u64)) as i32
    }

    fn f64(&mut self) -> f64 {
        (self.next() % 10_000) as f64 / 10_000.0
    }
}

pub struct Laseretch;

impl Laseretch {
    pub fn new() -> Self {
        Laseretch
    }
}

impl Effect for Laseretch {
    fn name(&self) -> &str {
        "laseretch"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;
        let mut rng = Rng::new(0x1a5e_e7c4_51ab_77d3);

        let num_input = terminal.characters.len();
        if num_input == 0 {
            return vec![terminal.get_formatted_output_string()];
        }

        // Palette (mirrors the upstream defaults in spirit).
        let white = Color::from_hex("FFFFFF").expect("valid hex");
        let beam_color = Color::from_hex("376CFF").expect("valid hex");
        let beam_dim = Color::from_hex("1F3C8C").expect("valid hex");
        let hot_orange = Color::from_hex("FF9600").expect("valid hex");
        let ember = Color::from_hex("8A2B00").expect("valid hex");
        let final_stops = [
            Color::from_hex("8A008A").expect("valid hex"),
            Color::from_hex("00D1FF").expect("valid hex"),
            white,
        ];
        let final_gradient = Gradient::new(&final_stops, 12);
        let spark_gradient = Gradient::new(&[white, hot_orange, ember], 3);

        // Build each input character's etch scene: white-hot block, then a
        // cooling ramp that lands on the character's final gradient color.
        for idx in 0..num_input {
            let ch = &mut terminal.characters[idx];
            let frac = if height > 1 {
                (height - ch.input_coord.row) as f64 / (height - 1) as f64
            } else {
                0.0
            };
            let final_color = final_gradient.get_color_at_fraction(frac).unwrap_or(white);
            let sym = ch.input_symbol;
            let cooling = Gradient::new(&[hot_orange, ember, final_color], 4);
            let scene = ch.animation.new_scene("etch", false);
            scene.add_frame('█', 3, ColorPair::fg(white), true);
            scene.add_frame('▓', 3, ColorPair::fg(hot_orange), true);
            for color in &cooling.spectrum {
                scene.add_frame(sym, 2, ColorPair::fg(*color), false);
            }
            scene.add_frame(sym, 1, ColorPair::fg(final_color), false);
        }

        // Coord -> input character index, plus the traversal span.
        let mut coord_map: HashMap<Coord, usize> = HashMap::new();
        let mut min_col = i32::MAX;
        let mut max_col = i32::MIN;
        let mut row_has = vec![false; (height as usize) + 1];
        for (idx, ch) in terminal.get_characters().iter().enumerate() {
            coord_map.insert(ch.input_coord, idx);
            min_col = min_col.min(ch.input_coord.column);
            max_col = max_col.max(ch.input_coord.column);
            row_has[ch.input_coord.row as usize] = true;
        }

        // Serpentine visit order: rows top to bottom, alternating direction.
        let mut visit: Vec<Coord> = Vec::new();
        let mut serp = 0usize;
        for row in (1..=height).rev() {
            if !row_has[row as usize] {
                continue;
            }
            if serp % 2 == 0 {
                for col in min_col..=max_col {
                    visit.push(Coord::new(col, row));
                }
            } else {
                for col in (min_col..=max_col).rev() {
                    visit.push(Coord::new(col, row));
                }
            }
            serp += 1;
        }

        // Extra arena characters: the laser head and a vertical beam feeding
        // it from the top of the canvas. Driven manually each tick.
        let mut next_id = 1_000_000usize;
        let head_idx = terminal.characters.len();
        terminal
            .characters
            .push(EffectCharacter::new(next_id, '█', Coord::new(1, height)));
        next_id += 1;
        let beam_start = terminal.characters.len();
        let beam_capacity = terminal.canvas.height;
        for _ in 0..beam_capacity {
            terminal
                .characters
                .push(EffectCharacter::new(next_id, '│', Coord::new(1, height)));
            next_id += 1;
        }

        let spark_symbols = ['*', '.', '+', '\''];
        let mut spark_indices: Vec<usize> = Vec::new();
        let mut etched = vec![false; num_input];
        let mut visit_idx = 0usize;
        let mut tick: u64 = 0;
        let mut frames_out: Vec<String> = Vec::new();

        loop {
            // Advance the laser head and etch anything it passes over.
            let laser_pos = if visit_idx < visit.len() {
                let mut last = visit[visit_idx];
                for _ in 0..ETCH_SPEED {
                    if visit_idx >= visit.len() {
                        break;
                    }
                    let coord = visit[visit_idx];
                    visit_idx += 1;
                    last = coord;
                    if let Some(&ci) = coord_map.get(&coord) {
                        if !etched[ci] {
                            etched[ci] = true;
                            terminal.characters[ci].is_visible = true;
                            terminal.characters[ci].animation.activate_scene("etch");

                            // Spawn a small burst of sparks arcing away.
                            let count = rng.range(1, 3);
                            for _ in 0..count {
                                let sym =
                                    spark_symbols[(rng.next() % spark_symbols.len() as u64) as usize];
                                let mut spark = EffectCharacter::new(next_id, sym, coord);
                                next_id += 1;
                                spark.is_visible = true;
                                let dx = rng.range(-3, 3);
                                let apex = Coord::new(
                                    (coord.column + dx).clamp(1, width),
                                    (coord.row + rng.range(1, 2)).min(height),
                                );
                                let land = Coord::new(
                                    (apex.column + rng.range(-1, 1)).clamp(1, width),
                                    (coord.row - rng.range(2, 5)).max(1),
                                );
                                let speed = 0.3 + rng.f64() * 0.4;
                                {
                                    let path =
                                        spark.motion.new_path("arc", speed, Some(easing::out_quad));
                                    path.add_waypoint(coord);
                                    path.add_waypoint(apex);
                                    path.add_waypoint(land);
                                }
                                spark.motion.activate_path("arc");
                                {
                                    let scene = spark.animation.new_scene("cool", false);
                                    for (i, color) in spark_gradient.spectrum.iter().enumerate() {
                                        scene.add_frame(sym, 3, ColorPair::fg(*color), i < 2);
                                    }
                                }
                                spark.animation.activate_scene("cool");
                                spark_indices.push(terminal.characters.len());
                                terminal.characters.push(spark);
                            }
                        }
                    }
                }
                Some(last)
            } else {
                None
            };

            // Draw (or hide) the laser head and its beam.
            match laser_pos {
                Some(pos) => {
                    let head_color = if tick % 2 == 0 { white } else { beam_color };
                    let head = &mut terminal.characters[head_idx];
                    head.is_visible = true;
                    head.motion.current_coord = pos;
                    head.animation.current_visual =
                        CharacterVisual::new('█', true, ColorPair::fg(head_color));
                    let beam_len = (height - pos.row).max(0) as usize;
                    for i in 0..beam_capacity {
                        let bc = &mut terminal.characters[beam_start + i];
                        if i < beam_len {
                            bc.is_visible = true;
                            bc.motion.current_coord =
                                Coord::new(pos.column, pos.row + 1 + i as i32);
                            let color = if (tick + i as u64) % 2 == 0 {
                                beam_color
                            } else {
                                beam_dim
                            };
                            bc.animation.current_visual =
                                CharacterVisual::new('│', false, ColorPair::fg(color));
                        } else {
                            bc.is_visible = false;
                        }
                    }
                }
                None => {
                    terminal.characters[head_idx].is_visible = false;
                    for i in 0..beam_capacity {
                        terminal.characters[beam_start + i].is_visible = false;
                    }
                }
            }

            terminal.tick();

            // Extinguish sparks that have finished cooling and landing.
            for &si in &spark_indices {
                if !terminal.characters[si].is_active() {
                    terminal.characters[si].is_visible = false;
                }
            }

            tick += 1;
            frames_out.push(terminal.get_formatted_output_string());

            if visit_idx >= visit.len() {
                let any_active = terminal.characters.iter().any(|c| c.is_active());
                if !any_active {
                    break;
                }
            }
            if frames_out.len() >= MAX_FRAMES {
                break;
            }
        }

        // Hold the finished etch briefly.
        if let Some(last) = frames_out.last().cloned() {
            for _ in 0..10 {
                frames_out.push(last.clone());
            }
        }

        frames_out
    }
}
