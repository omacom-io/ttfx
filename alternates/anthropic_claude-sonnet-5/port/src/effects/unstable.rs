//! Port of `effect_unstable.py`: characters explode outward to a random
//! point on the edge of the canvas, rumble/glitch in place for a while, then
//! reassemble back into their original position and appearance.
//!
//! The engine's `Motion`/`Path` stepping (see `engine/motion.rs`) always
//! resolves to the first waypoint of whatever path is active, so this port
//! drives position directly via the public `motion.current_pos` /
//! `motion.current_coord` fields (interpolated with `utils::geometry::lerp`
//! and eased with `utils::easing`) rather than through `Path`/`Motion::step`.

use super::Effect;
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::{self, Coord};
use crate::utils::graphics::{Color, ColorPair};

const EXPLOSION_TICKS: usize = 20;
const RUMBLE_TICKS: usize = 12;
const REASSEMBLY_TICKS: usize = 20;
const UNSTABLE_COLOR: Color = Color::Rgb(255, 159, 0);
const JUMBLE_SYMBOLS: [char; 8] = ['!', '@', '#', '$', '%', '^', '&', '*'];

/// Small deterministic xorshift64* PRNG. The engine skeleton has no shared
/// `rng` module yet (see plan.md's `utils/rng.rs`, not present in this
/// crate), so this effect carries its own minimal generator rather than
/// inventing a new crate module.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        if x == 0 {
            x = 0xDEAD_BEEF_CAFE_F00D;
        }
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Inclusive range on both ends, mirroring Python's `random.randint`.
    fn gen_range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i32
    }

    fn choice_char(&mut self, items: &[char]) -> char {
        let idx = (self.next_u64() as usize) % items.len();
        items[idx]
    }
}

/// Explosion -> rumble -> reassembly effect.
pub struct Unstable;

impl Unstable {
    pub fn new() -> Self {
        Unstable
    }

    /// Pick a random point on the canvas edge, matching the Python
    /// original's four-sided edge selection in `effect_unstable.py`.
    fn explosion_target(rng: &mut Rng, width: i32, height: i32) -> Coord {
        let left = 0;
        let right = (width - 1).max(0);
        let top = 0;
        let bottom = (height - 1).max(0);
        match rng.gen_range(0, 3) {
            0 => Coord::new(left, rng.gen_range(top, bottom)),
            1 => Coord::new(right, rng.gen_range(top, bottom)),
            2 => Coord::new(rng.gen_range(left, right), bottom),
            _ => Coord::new(rng.gen_range(left, right), top),
        }
    }
}

impl Effect for Unstable {
    fn name(&self) -> &str {
        "unstable"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;

        let char_count = terminal.get_characters().len();
        let mut rng = Rng::new(0x5EED_1234 ^ (char_count as u64));
        let mut targets: Vec<Coord> = Vec::with_capacity(char_count);
        for _ in 0..char_count {
            targets.push(Self::explosion_target(&mut rng, width, height));
        }

        let total_ticks = EXPLOSION_TICKS + RUMBLE_TICKS + REASSEMBLY_TICKS;
        let mut frames = Vec::with_capacity(total_ticks + 1);

        for tick in 0..=total_ticks {
            for (idx, character) in terminal.get_characters_mut().iter_mut().enumerate() {
                let origin = character.input_coord;
                let target = targets[idx];

                if tick < EXPLOSION_TICKS {
                    let t = tick as f64 / EXPLOSION_TICKS as f64;
                    let eased_t = easing::ease_out_cubic(t);
                    let (x, y) = geometry::lerp(origin, target, eased_t);
                    character.motion.current_pos = (x, y);
                    character.motion.current_coord = Coord::new(x.round() as i32, y.round() as i32);
                    character.animation.set_appearance(
                        character.input_symbol,
                        Some(ColorPair::new(Some(UNSTABLE_COLOR), None)),
                    );
                } else if tick < EXPLOSION_TICKS + RUMBLE_TICKS {
                    let rumble_index = tick - EXPLOSION_TICKS;
                    let mut jitter_rng =
                        Rng::new((idx as u64).wrapping_mul(2654435761) ^ rumble_index as u64);
                    let dx = jitter_rng.gen_range(-1, 1);
                    let dy = jitter_rng.gen_range(-1, 1);
                    let jittered = Coord::new(target.column + dx, target.row + dy);
                    character.motion.current_pos = (jittered.column as f64, jittered.row as f64);
                    character.motion.current_coord = jittered;
                    let symbol = if rumble_index % 2 == 0 {
                        jitter_rng.choice_char(&JUMBLE_SYMBOLS)
                    } else {
                        character.input_symbol
                    };
                    character.animation.set_appearance(
                        symbol,
                        Some(ColorPair::new(Some(UNSTABLE_COLOR), None)),
                    );
                } else {
                    let reassembly_tick = tick - EXPLOSION_TICKS - RUMBLE_TICKS;
                    let t = (reassembly_tick as f64 / REASSEMBLY_TICKS as f64).min(1.0);
                    let eased_t = easing::ease_in_out_sine(t);
                    let (x, y) = geometry::lerp(target, origin, eased_t);
                    character.motion.current_pos = (x, y);
                    character.motion.current_coord = Coord::new(x.round() as i32, y.round() as i32);
                    if t >= 1.0 {
                        character.animation.set_appearance(character.input_symbol, None);
                    } else {
                        character.animation.set_appearance(
                            character.input_symbol,
                            Some(ColorPair::new(Some(UNSTABLE_COLOR), None)),
                        );
                    }
                }
            }
            frames.push(terminal.render());
        }

        frames
    }
}
