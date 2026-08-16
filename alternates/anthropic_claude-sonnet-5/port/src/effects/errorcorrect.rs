use super::Effect;
use crate::engine::terminal::Terminal;
use crate::utils::geometry::{self, Coord};
use crate::utils::graphics::{Color, ColorPair};

/// Minimal deterministic PRNG (xorshift64) used to choose pairing and
/// correction order. No shared `rng` module is available to this effect, so
/// a tiny self-contained generator stands in for `random`.
struct SimpleRng(u64);

impl SimpleRng {
    fn new(seed: u64) -> Self {
        SimpleRng(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        let len = items.len();
        if len < 2 {
            return;
        }
        for i in (1..len).rev() {
            let j = (self.next_u64() as usize) % (i + 1);
            items.swap(i, j);
        }
    }
}

const ERROR_COLOR: Color = Color::Rgb(0xe7, 0x4c, 0x3c);
const HOLD_FRAMES: usize = 12;
const CORRECT_FRAMES: usize = 8;
const GAP_FRAMES: usize = 2;
const FINAL_HOLD_FRAMES: usize = 20;

/// Places characters at randomly swapped "incorrect" positions, flagged with
/// an error color, then corrects them one pair at a time back to their home
/// coordinates -- mirrors `terminaltexteffects/effects/effect_errorcorrect.py`
/// at the level supported by this engine skeleton (direct position/visual
/// mutation rather than `Motion::step`/`Path`, since a multi-waypoint `Path`
/// here never advances past its zero-length anchor segment).
pub struct Errorcorrect;

impl Errorcorrect {
    pub fn new() -> Self {
        Errorcorrect
    }
}

impl Effect for Errorcorrect {
    fn name(&self) -> &str {
        "errorcorrect"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let mut rng = SimpleRng::new(0x2545_F491_4F6C_DD1D);

        // Gather non-space character ids in original (input) order; spaces
        // are left untouched, matching upstream's skip-whitespace behavior.
        let mut movable_ids: Vec<u32> = terminal
            .get_characters()
            .iter()
            .filter(|c| c.input_symbol != ' ')
            .map(|c| c.id)
            .collect();

        // Shuffle into a random order, then pair up adjacent ids. An odd
        // leftover id (if any) is simply excluded from `pairs` and stays at
        // its untouched, correctly-placed default appearance.
        rng.shuffle(&mut movable_ids);
        let mut pairs: Vec<(u32, u32)> = Vec::new();
        for chunk in movable_ids.chunks_exact(2) {
            pairs.push((chunk[0], chunk[1]));
        }

        // Swap each pair's displayed starting position and mark both with
        // the error color, simulating the "misplaced" initial state.
        for &(a, b) in &pairs {
            let coord_a = terminal.get_character(a).unwrap().input_coord;
            let coord_b = terminal.get_character(b).unwrap().input_coord;
            if let Some(ca) = terminal.get_character_mut(a) {
                ca.motion.current_coord = coord_b;
                ca.motion.current_pos = (coord_b.column as f64, coord_b.row as f64);
                let symbol = ca.input_symbol;
                ca.animation
                    .set_appearance(symbol, Some(ColorPair::new(Some(ERROR_COLOR), None)));
            }
            if let Some(cb) = terminal.get_character_mut(b) {
                cb.motion.current_coord = coord_a;
                cb.motion.current_pos = (coord_a.column as f64, coord_a.row as f64);
                let symbol = cb.input_symbol;
                cb.animation
                    .set_appearance(symbol, Some(ColorPair::new(Some(ERROR_COLOR), None)));
            }
        }

        let mut frames: Vec<String> = Vec::new();

        // Hold on the fully-scrambled, error-colored state.
        for _ in 0..HOLD_FRAMES {
            frames.push(terminal.render());
        }

        // Correct one pair at a time, in a freshly shuffled order, gradually
        // moving each half of the pair back to its home coordinate and
        // clearing the error color once it arrives.
        let mut correction_order = pairs.clone();
        rng.shuffle(&mut correction_order);

        for &(a, b) in &correction_order {
            let home_a = terminal.get_character(a).unwrap().input_coord;
            let home_b = terminal.get_character(b).unwrap().input_coord;
            let start_a = terminal.get_character(a).unwrap().motion.current_coord;
            let start_b = terminal.get_character(b).unwrap().motion.current_coord;

            for step in 1..=CORRECT_FRAMES {
                let t = step as f64 / CORRECT_FRAMES as f64;
                let (xa, ya) = geometry::lerp(start_a, home_a, t);
                let (xb, yb) = geometry::lerp(start_b, home_b, t);
                let arrived = step == CORRECT_FRAMES;

                if let Some(ca) = terminal.get_character_mut(a) {
                    ca.motion.current_pos = (xa, ya);
                    ca.motion.current_coord = Coord::new(xa.round() as i32, ya.round() as i32);
                    let symbol = ca.input_symbol;
                    if arrived {
                        ca.animation.set_appearance(symbol, None);
                    } else {
                        ca.animation
                            .set_appearance(symbol, Some(ColorPair::new(Some(ERROR_COLOR), None)));
                    }
                }
                if let Some(cb) = terminal.get_character_mut(b) {
                    cb.motion.current_pos = (xb, yb);
                    cb.motion.current_coord = Coord::new(xb.round() as i32, yb.round() as i32);
                    let symbol = cb.input_symbol;
                    if arrived {
                        cb.animation.set_appearance(symbol, None);
                    } else {
                        cb.animation
                            .set_appearance(symbol, Some(ColorPair::new(Some(ERROR_COLOR), None)));
                    }
                }

                frames.push(terminal.render());
            }

            for _ in 0..GAP_FRAMES {
                frames.push(terminal.render());
            }
        }

        // Final hold on the fully-corrected text.
        for _ in 0..FINAL_HOLD_FRAMES {
            frames.push(terminal.render());
        }

        frames
    }
}
