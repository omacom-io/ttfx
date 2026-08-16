use super::Effect;
use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::motion::{Path, Segment, Waypoint};
use crate::engine::terminal::Terminal;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair};

/// Glyphs used for the falling "flicker" frames before a character settles
/// into its final input symbol, approximating the katakana rain glyphs used
/// upstream with a portable ASCII set.
const MATRIX_GLYPHS: [char; 12] = ['0', '1', '7', '$', '%', '#', '@', '*', '+', '=', '~', '^'];

/// A tiny deterministic splitmix64-style pseudo-random generator, used in
/// place of the upstream `random` module since no shared RNG utility exists
/// in this crate yet. Seeded per-character by id so runs are reproducible.
fn pseudo_rand(seed: u64) -> u64 {
    let mut x = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    x
}

fn visual_with_color(symbol: char, colors: Option<ColorPair>) -> CharacterVisual {
    let mut visual = CharacterVisual::new(symbol);
    visual.colors = colors;
    visual.formatted_symbol = visual.format_symbol();
    visual
}

/// Matrix rain effect: characters fall from above the canvas into their
/// input position, flickering through random glyphs before settling into a
/// dim green rendering of their true symbol. Mirrors the overall shape of
/// `terminaltexteffects/effects/effect_matrix.py`'s rain/resolve behavior,
/// simplified to the primitives available in this engine skeleton.
pub struct Matrix;

impl Matrix {
    pub fn new() -> Self {
        Matrix
    }
}

impl Effect for Matrix {
    fn name(&self) -> &str {
        "matrix"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let height = terminal.config.height as i32;

        let highlight_color = Color::Rgb(200, 255, 200);
        let bright_green = Color::Rgb(0, 255, 70);
        let mid_green = Color::Rgb(0, 180, 60);
        let dark_green = Color::Rgb(0, 100, 40);

        let ids: Vec<u32> = terminal.get_characters().iter().map(|c| c.id).collect();

        for &id in &ids {
            let (input_coord, input_symbol) = {
                let ch = terminal.get_character(id).unwrap();
                (ch.input_coord, ch.input_symbol)
            };

            if input_symbol == ' ' {
                continue;
            }

            let r1 = pseudo_rand(id as u64 * 2 + 1);
            let extra_rows = (r1 % (height as u64 + 1)) as i32;
            let start_row = input_coord.row - height - extra_rows - 1;
            let start_coord = Coord::new(input_coord.column, start_row);

            let speed = 0.6 + ((r1 % 100) as f64) / 200.0; // 0.6 .. 1.1

            // Build the fall path with a single real segment, bypassing
            // Path::add_waypoint's zero-distance anchor segment (which the
            // motion stepping loop always matches first, freezing motion).
            let mut fall_path = Path::new("fall", speed);
            fall_path
                .segments
                .push(Segment::new(Waypoint::new(start_coord), Waypoint::new(input_coord)));

            let mut resolve_scene = Scene::new("resolve");
            resolve_scene.is_looping = false;
            resolve_scene.add_frame(
                visual_with_color(input_symbol, Some(ColorPair::new(Some(highlight_color), None))),
                1,
            );
            let flicker_count = 2 + (r1 % 3) as u32; // 2..4
            for i in 0..flicker_count {
                let glyph_idx = ((r1 >> (i + 3)) % MATRIX_GLYPHS.len() as u64) as usize;
                let glyph = MATRIX_GLYPHS[glyph_idx];
                resolve_scene.add_frame(
                    visual_with_color(glyph, Some(ColorPair::new(Some(bright_green), None))),
                    1,
                );
            }
            resolve_scene.add_frame(
                visual_with_color(input_symbol, Some(ColorPair::new(Some(mid_green), None))),
                2,
            );
            resolve_scene.add_frame(
                visual_with_color(input_symbol, Some(ColorPair::new(Some(dark_green), None))),
                1000,
            );

            let ch = terminal.get_character_mut(id).unwrap();
            ch.motion.current_coord = start_coord;
            ch.motion.current_pos = (start_coord.column as f64, start_coord.row as f64);
            ch.motion.add_path(fall_path);
            ch.motion.activate_path("fall");
            ch.animation.add_scene(resolve_scene);
        }

        let max_ticks = (height as usize) * 4 + 30;
        let mut resolved: Vec<bool> = vec![false; ids.len()];
        let mut frames = Vec::with_capacity(max_ticks);

        for _ in 0..max_ticks {
            for (idx, &id) in ids.iter().enumerate() {
                if resolved[idx] {
                    continue;
                }
                let arrived = {
                    let ch = terminal.get_character(id).unwrap();
                    ch.motion.current_coord == ch.input_coord
                };
                if arrived {
                    resolved[idx] = true;
                    let ch = terminal.get_character_mut(id).unwrap();
                    ch.animation.activate_scene("resolve");
                }
            }
            terminal.step_animation();
            frames.push(terminal.render());
        }

        frames
    }
}
