use super::Effect;
use crate::engine::terminal::Terminal;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Rain: characters fall from above the canvas down into their input
/// position, rendered as blue-toned rain-drop symbols while falling and
/// settling into a bright landed color once they reach their final spot.
/// Mirrors the shape of `terminaltexteffects/effects/effect_rain.py`
/// (staggered per-character fall delay/speed, gradient rain coloring,
/// gradient landed coloring), simplified to the primitives exposed by this
/// engine skeleton (direct `Motion`/`Animation` field manipulation rather
/// than the upstream event-handler-driven scene/path activation).
pub struct Rain;

impl Rain {
    pub fn new() -> Self {
        Rain
    }
}

/// Small deterministic integer hash (xorshift-multiply mix), used in place
/// of `utils::rng` (not part of this skeleton's file set) to derive
/// per-character stagger/speed/symbol/color variety from the character id.
fn hash_u32(x: u32) -> u32 {
    let mut h = x.wrapping_mul(2654435761);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h ^= h >> 7;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;
    h
}

struct Meta {
    delay: i32,
    duration: i32,
    symbol: char,
    rain_color_idx: usize,
    final_color: Color,
}

impl Effect for Rain {
    fn name(&self) -> &str {
        "rain"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let height = terminal.config.height as i32;

        // Rain-drop color gradient (dark blue -> pale blue), mirroring the
        // upstream rain_colors palette.
        let rain_stops = [
            Color::Rgb(0x00, 0x31, 0x5C),
            Color::Rgb(0x00, 0x4C, 0x8F),
            Color::Rgb(0x00, 0x75, 0xDB),
            Color::Rgb(0x3F, 0x91, 0xD9),
            Color::Rgb(0x78, 0xB9, 0xF2),
            Color::Rgb(0x9A, 0xC8, 0xF5),
            Color::Rgb(0xB8, 0xD8, 0xF8),
            Color::Rgb(0xE3, 0xEF, 0xFC),
        ];
        let rain_gradient = Gradient::new(&rain_stops, 6);

        // Landed color gradient (magenta -> cyan -> white), mirroring the
        // upstream final_gradient_stops.
        let final_stops = [
            Color::Rgb(0x8A, 0x00, 0x8A),
            Color::Rgb(0x00, 0xD1, 0xFF),
            Color::Rgb(0xFF, 0xFF, 0xFF),
        ];
        let final_gradient = Gradient::new(&final_stops, 8);

        let rain_symbols = ['|', '.', ',', '*', '`', '\''];

        let mut metas: Vec<Meta> = Vec::with_capacity(terminal.get_characters().len());
        for character in terminal.get_characters().iter() {
            let h = hash_u32(character.id.wrapping_add(1));
            let delay = (h % 40) as i32;
            let duration = 6 + ((h >> 4) % 10) as i32;
            let symbol = rain_symbols[(h as usize >> 3) % rain_symbols.len()];
            let rain_color_idx = (h as usize >> 6) % rain_gradient.len().max(1);
            let final_color = final_gradient
                .get((h as usize >> 2) % final_gradient.len().max(1))
                .unwrap_or(Color::Rgb(0xFF, 0xFF, 0xFF));
            metas.push(Meta { delay, duration, symbol, rain_color_idx, final_color });
        }

        // Reposition every non-space character above the canvas, staggered
        // by its delay, so it starts off-screen and falls into place.
        for (character, meta) in terminal.get_characters_mut().iter_mut().zip(metas.iter()) {
            if character.input_symbol == ' ' {
                continue;
            }
            let start_row = character.input_coord.row - height - meta.delay;
            character.motion.current_coord = Coord::new(character.input_coord.column, start_row);
            character.motion.current_pos = (character.input_coord.column as f64, start_row as f64);
        }

        let max_delay = metas.iter().map(|m| m.delay).max().unwrap_or(0);
        let max_duration = metas.iter().map(|m| m.duration).max().unwrap_or(1);
        let total_ticks = max_delay + max_duration + height + 5;

        let mut frames = Vec::with_capacity(total_ticks.max(1) as usize);
        for tick in 0..total_ticks {
            for (character, meta) in terminal.get_characters_mut().iter_mut().zip(metas.iter()) {
                if character.input_symbol == ' ' {
                    continue;
                }
                let elapsed = tick - meta.delay;
                if elapsed < 0 {
                    // Not yet released; keep parked off-screen above the canvas.
                    continue;
                }
                if elapsed >= meta.duration {
                    character.motion.current_coord = character.input_coord;
                    character.motion.current_pos =
                        (character.input_coord.column as f64, character.input_coord.row as f64);
                    character
                        .animation
                        .set_appearance(character.input_symbol, Some(ColorPair::new(Some(meta.final_color), None)));
                } else {
                    let t = elapsed as f64 / meta.duration as f64;
                    let start_row = character.input_coord.row - height - meta.delay;
                    let row = start_row as f64 + (character.input_coord.row as f64 - start_row as f64) * t;
                    character.motion.current_coord = Coord::new(character.input_coord.column, row.round() as i32);
                    character.motion.current_pos = (character.input_coord.column as f64, row);
                    let color = rain_gradient
                        .get(meta.rain_color_idx)
                        .unwrap_or(Color::Rgb(0x00, 0x75, 0xDB));
                    character
                        .animation
                        .set_appearance(meta.symbol, Some(ColorPair::new(Some(color), None)));
                }
            }
            frames.push(terminal.render());
        }

        frames
    }
}
