//! Bouncyballs effect: characters fall from above the canvas onto their
//! input position like bouncing balls, showing a ball symbol/color while
//! falling and settling into their true symbol once landed. Mirrors the
//! spirit of `terminaltexteffects/effects/effect_bouncyballs.py`, adapted to
//! the engine primitives actually available in this skeleton (no event
//! handler / RNG module yet, so timing and color selection are derived
//! deterministically from each character's arena id).

use super::Effect;
use crate::engine::terminal::Terminal;
use crate::utils::easing::ease_out_cubic;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair};

const BALL_SYMBOLS: [char; 5] = ['*', 'o', 'O', '.', '+'];
const BALL_COLORS: [Color; 6] = [
    Color::Rgb(255, 0, 0),
    Color::Rgb(255, 165, 0),
    Color::Rgb(255, 255, 0),
    Color::Rgb(0, 200, 0),
    Color::Rgb(0, 191, 255),
    Color::Rgb(178, 102, 255),
];

pub struct Bouncyballs;

impl Bouncyballs {
    pub fn new() -> Self {
        Bouncyballs
    }
}

struct Fall {
    id: u32,
    column: i32,
    start_row: i32,
    target_row: i32,
    delay: usize,
    duration: usize,
    symbol: char,
    color: Color,
}

impl Effect for Bouncyballs {
    fn name(&self) -> &str {
        "bouncyballs"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let canvas_height = terminal.config.height as i32;

        let mut falls: Vec<Fall> = Vec::new();
        for character in terminal.get_characters() {
            if character.input_symbol == ' ' {
                continue;
            }
            let idx = character.id as usize;
            let symbol = BALL_SYMBOLS[idx % BALL_SYMBOLS.len()];
            let color = BALL_COLORS[idx % BALL_COLORS.len()];
            let stagger = idx % 12;
            let start_row = -(canvas_height + stagger as i32 + 1);
            let target_row = character.input_coord.row;
            let fall_distance = (target_row - start_row).unsigned_abs() as usize;
            let duration = (fall_distance / 2).max(6);

            falls.push(Fall {
                id: character.id,
                column: character.input_coord.column,
                start_row,
                target_row,
                delay: stagger * 2,
                duration,
                symbol,
                color,
            });
        }

        let max_ticks = falls
            .iter()
            .map(|f| f.delay + f.duration)
            .max()
            .unwrap_or(0)
            + 10;

        let mut frames = Vec::with_capacity(max_ticks);
        for tick in 0..max_ticks {
            for fall in &falls {
                let character = match terminal.get_character_mut(fall.id) {
                    Some(c) => c,
                    None => continue,
                };
                let landed_symbol = character.input_symbol;

                if tick < fall.delay {
                    character.motion.current_coord = Coord::new(fall.column, fall.start_row);
                    character
                        .animation
                        .set_appearance(fall.symbol, Some(ColorPair::new(Some(fall.color), None)));
                    continue;
                }

                let elapsed = tick - fall.delay;
                if elapsed >= fall.duration {
                    character.motion.current_coord = Coord::new(fall.column, fall.target_row);
                    character.animation.set_appearance(
                        landed_symbol,
                        Some(ColorPair::new(Some(fall.color), None)),
                    );
                } else {
                    let t = elapsed as f64 / fall.duration as f64;
                    let eased = ease_out_cubic(t);
                    let row =
                        fall.start_row as f64 + (fall.target_row - fall.start_row) as f64 * eased;
                    character.motion.current_coord = Coord::new(fall.column, row.round() as i32);
                    character
                        .animation
                        .set_appearance(fall.symbol, Some(ColorPair::new(Some(fall.color), None)));
                }
            }

            frames.push(terminal.render());
        }

        frames
    }
}
