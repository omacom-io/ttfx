//! Bouncy Balls effect: characters fall from above the canvas as randomly
//! colored "balls", bounce into their input position, then fade from the
//! ball color to their final gradient color.
//!
//! Port of terminaltexteffects/effects/effect_bouncyballs.py.

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing::EasingFn;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Default ball colors from the Python effect config.
const BALL_COLORS: [&str; 3] = ["d1f4a5", "96e2a4", "5acda9"];
/// Default ball symbols from the Python effect config.
const BALL_SYMBOLS: [char; 5] = ['*', 'o', 'O', '0', '.'];
/// Default final gradient stops from the Python effect config.
const FINAL_GRADIENT_STOPS: [&str; 3] = ["8A008A", "00D1FF", "FFFFFF"];
/// Ticks between bursts of released balls (`--ball-delay`).
const BALL_DELAY: u32 = 7;
/// Movement speed of falling characters (`--movement-speed`).
const MOVEMENT_SPEED: f64 = 0.25;
/// Steps in the per-character fade gradient (ball color -> final color).
const FADE_STEPS: usize = 10;
/// Duration (ticks) of each fade frame.
const FADE_FRAME_DURATION: u32 = 10;

/// The Python default `--movement-easing` is OUT_BOUNCE, which is not in the
/// shared easing module, so it is defined locally.
fn out_bounce(t: f64) -> f64 {
    let n1 = 7.5625;
    let d1 = 2.75;
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

/// Small xorshift PRNG standing in for Python's `random` module.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Rng {
            state: seed | 1, // never zero
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// random.uniform(a, b)
    fn uniform(&mut self, a: f64, b: f64) -> f64 {
        let f = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        a + (b - a) * f
    }

    /// random.randint(lo, hi) — inclusive on both ends.
    fn randint(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() as usize) % (hi - lo + 1)
    }

    /// random.choice(items)
    fn choice<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.randint(0, items.len() - 1)]
    }
}

pub struct Bouncyballs;

impl Bouncyballs {
    pub fn new() -> Self {
        Bouncyballs
    }
}

impl Default for Bouncyballs {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Bouncyballs {
    fn name(&self) -> &str {
        "bouncyballs"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());

        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5eed_ba11);
        let mut rng = Rng::new(seed);

        let ball_colors: Vec<Color> = BALL_COLORS
            .iter()
            .map(|hex| Color::from_hex(hex).expect("valid ball color"))
            .collect();
        let final_stops: Vec<Color> = FINAL_GRADIENT_STOPS
            .iter()
            .map(|hex| Color::from_hex(hex).expect("valid gradient stop"))
            .collect();
        let final_gradient = Gradient::new(&final_stops, 12);

        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;
        let char_count = terminal.get_characters().len();

        // ---- build (Python: BouncyBallsIterator.build) ----
        for ch in terminal.get_characters_mut() {
            let ball_color = *rng.choice(&ball_colors);
            let ball_symbol = *rng.choice(&BALL_SYMBOLS);

            // Final color mapped across the canvas by column.
            let fraction = if width > 1 {
                (ch.input_coord.column - 1) as f64 / (width - 1) as f64
            } else {
                0.0
            };
            let final_color = final_gradient
                .get_color_at_fraction(fraction)
                .unwrap_or(ball_color);

            // Ball scene: the falling symbol in its ball color.
            {
                let ball_scene = ch.animation.new_scene("ball", false);
                ball_scene.add_frame(ball_symbol, 1, ColorPair::fg(ball_color), false);
            }

            // Final scene: fade from the ball color to the final gradient color
            // while showing the input symbol.
            {
                let fade = Gradient::new(&[ball_color, final_color], FADE_STEPS);
                let final_scene = ch.animation.new_scene("final", false);
                let input_symbol = ch.input_symbol;
                for color in &fade.spectrum {
                    final_scene.add_frame(
                        input_symbol,
                        FADE_FRAME_DURATION,
                        ColorPair::fg(*color),
                        false,
                    );
                }
            }

            // Start above the canvas top at a random height (Python:
            // int(canvas.top * random.uniform(1.0, 1.5))).
            let start_row = (height as f64 * rng.uniform(1.0, 1.5)) as i32;
            let start_coord = Coord::new(ch.input_coord.column, start_row);
            ch.motion.current_coord = start_coord;

            let input_coord = ch.input_coord;
            let path = ch
                .motion
                .new_path("input_coord", MOVEMENT_SPEED, Some(out_bounce as EasingFn));
            path.add_waypoint(start_coord);
            path.add_waypoint(input_coord);
        }

        let mut pending: Vec<usize> = terminal
            .get_characters()
            .iter()
            .map(|c| c.character_id)
            .collect();
        let mut finalized = vec![false; char_count];

        let mut frames: Vec<String> = Vec::new();
        let mut delay: u32 = 0;
        let mut active: usize = 0;
        let mut guard: usize = 0;

        // ---- animate (Python: BouncyBallsIterator.__next__) ----
        while (!pending.is_empty() || active > 0) && guard < 100_000 {
            guard += 1;

            if !pending.is_empty() {
                if delay == 0 {
                    // Release a burst of 6..=15 randomly chosen balls.
                    let burst = rng.randint(6, 15);
                    for _ in 0..burst {
                        if pending.is_empty() {
                            break;
                        }
                        let idx = rng.randint(0, pending.len() - 1);
                        let id = pending.swap_remove(idx);
                        if let Some(ch) = terminal
                            .characters
                            .iter_mut()
                            .find(|c| c.character_id == id)
                        {
                            ch.is_visible = true;
                            ch.animation.activate_scene("ball");
                            ch.motion.activate_path("input_coord");
                        }
                    }
                    delay = BALL_DELAY;
                } else {
                    delay -= 1;
                }
            }

            terminal.tick();

            // Path complete -> activate the final fade scene
            // (Python: PATH_COMPLETE event -> ACTIVATE_SCENE final_scene).
            for ch in terminal.get_characters_mut() {
                let id = ch.character_id;
                if id < finalized.len()
                    && !finalized[id]
                    && ch.is_visible
                    && ch.motion.movement_is_complete()
                {
                    let path_done = ch
                        .motion
                        .query_path("input_coord")
                        .map(|p| p.is_complete())
                        .unwrap_or(false);
                    if path_done {
                        ch.animation.activate_scene("final");
                        finalized[id] = true;
                    }
                }
            }

            active = terminal
                .get_characters()
                .iter()
                .filter(|c| c.is_active())
                .count();

            frames.push(terminal.get_formatted_output_string());
        }

        frames
    }
}
