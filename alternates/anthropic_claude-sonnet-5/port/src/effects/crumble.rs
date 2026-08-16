use super::Effect;

use crate::engine::character::CharacterId;
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::{self, Coord};
use crate::utils::graphics::{Color, ColorPair};

/// Minimal deterministic PRNG (xorshift32) used only to stagger per-character
/// timing. This crate has no `rng.rs` helper module, so effects that need
/// randomness roll their own small generator rather than depending on an
/// external crate.
struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Rng(seed.max(1))
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// Random integer in `[lo, hi)`.
    fn gen_range(&mut self, lo: u32, hi: u32) -> u32 {
        if hi <= lo {
            return lo;
        }
        lo + self.next_u32() % (hi - lo)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Waiting,
    Weakening,
    Falling,
    Rising,
    Landing,
    Done,
}

struct CState {
    input_coord: Coord,
    symbol: char,
    wait: i32,
    weaken_ticks: i32,
    fall_total: i32,
    fall_step: i32,
    rise_total: i32,
    rise_step: i32,
    land_total: i32,
    land_step: i32,
    phase: Phase,
}

/// The `crumble` effect: characters weaken (dim in color), collapse off the
/// bottom of the canvas, reappear at the top, then fall back into place at
/// their original position/appearance. This is a simplified re-derivation of
/// `terminaltexteffects/effects/effect_crumble.py`'s fall -> top -> home
/// choreography, driving `motion.current_coord`/`current_pos` directly each
/// tick (bezier control points and gradient-to-symbol scenes aren't
/// available on this port's reduced `Motion`/`Animation` surface).
pub struct Crumble;

impl Crumble {
    pub fn new() -> Self {
        Crumble
    }
}

impl Effect for Crumble {
    fn name(&self) -> &str {
        "crumble"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let mut rng = Rng::new(0x5EED_1234);

        let bottom_row = terminal.canvas.height as i32 - 1;
        let top_row = 0i32;

        let ids: Vec<CharacterId> = terminal.get_characters().iter().map(|c| c.id).collect();

        let mut states: Vec<CState> = Vec::with_capacity(ids.len());
        for &id in &ids {
            let character = terminal.get_character(id).unwrap();
            let input_coord = character.input_coord;
            let symbol = character.input_symbol;

            let wait = rng.gen_range(0, 40) as i32;
            let weaken_ticks = rng.gen_range(3, 8) as i32;

            let fall_target = Coord::new(input_coord.column, bottom_row);
            let rise_target = Coord::new(input_coord.column, top_row);

            let fall_dist = geometry::distance(input_coord, fall_target).max(1.0);
            let rise_dist = geometry::distance(fall_target, rise_target).max(1.0);
            let land_dist = geometry::distance(rise_target, input_coord).max(1.0);

            states.push(CState {
                input_coord,
                symbol,
                wait,
                weaken_ticks,
                fall_total: fall_dist.round() as i32,
                fall_step: 0,
                rise_total: rise_dist.round() as i32,
                rise_step: 0,
                land_total: land_dist.round() as i32,
                land_step: 0,
                phase: Phase::Waiting,
            });
        }

        let weak_color = Color::Rgb(90, 90, 90);
        let final_color = Color::Rgb(220, 220, 220);

        let mut frames = Vec::new();
        let max_ticks = 500usize;

        for _tick in 0..max_ticks {
            let mut any_active = false;

            for (idx, &id) in ids.iter().enumerate() {
                let st = &mut states[idx];
                match st.phase {
                    Phase::Waiting => {
                        any_active = true;
                        if st.wait > 0 {
                            st.wait -= 1;
                        } else {
                            let character = terminal.get_character_mut(id).unwrap();
                            character
                                .animation
                                .set_appearance(st.symbol, Some(ColorPair::new(Some(weak_color), None)));
                            st.phase = Phase::Weakening;
                        }
                    }
                    Phase::Weakening => {
                        any_active = true;
                        if st.weaken_ticks > 0 {
                            st.weaken_ticks -= 1;
                        } else {
                            st.phase = Phase::Falling;
                        }
                    }
                    Phase::Falling => {
                        any_active = true;
                        st.fall_step += 1;
                        let t = (st.fall_step as f64 / st.fall_total as f64).min(1.0);
                        let eased = easing::ease_out_cubic(t);
                        let fall_target = Coord::new(st.input_coord.column, bottom_row);
                        let (x, y) = geometry::lerp(st.input_coord, fall_target, eased);
                        let character = terminal.get_character_mut(id).unwrap();
                        character.motion.current_pos = (x, y);
                        character.motion.current_coord = Coord::new(x.round() as i32, y.round() as i32);
                        if t >= 1.0 {
                            st.phase = Phase::Rising;
                        }
                    }
                    Phase::Rising => {
                        any_active = true;
                        st.rise_step += 1;
                        let t = (st.rise_step as f64 / st.rise_total as f64).min(1.0);
                        let eased = easing::ease_out_expo(t);
                        let fall_target = Coord::new(st.input_coord.column, bottom_row);
                        let rise_target = Coord::new(st.input_coord.column, top_row);
                        let (x, y) = geometry::lerp(fall_target, rise_target, eased);
                        let character = terminal.get_character_mut(id).unwrap();
                        character.motion.current_pos = (x, y);
                        character.motion.current_coord = Coord::new(x.round() as i32, y.round() as i32);
                        if t >= 1.0 {
                            st.phase = Phase::Landing;
                        }
                    }
                    Phase::Landing => {
                        any_active = true;
                        st.land_step += 1;
                        let t = (st.land_step as f64 / st.land_total as f64).min(1.0);
                        let eased = easing::ease_out_cubic(t);
                        let rise_target = Coord::new(st.input_coord.column, top_row);
                        let (x, y) = geometry::lerp(rise_target, st.input_coord, eased);
                        let character = terminal.get_character_mut(id).unwrap();
                        character.motion.current_pos = (x, y);
                        character.motion.current_coord = Coord::new(x.round() as i32, y.round() as i32);
                        if t >= 1.0 {
                            character
                                .animation
                                .set_appearance(st.symbol, Some(ColorPair::new(Some(final_color), None)));
                            st.phase = Phase::Done;
                        }
                    }
                    Phase::Done => {}
                }
            }

            frames.push(terminal.render());

            if !any_active {
                break;
            }
        }

        frames
    }
}
