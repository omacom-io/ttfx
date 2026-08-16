//! Movie-style decryption effect.
//!
//! Characters are typed onto the canvas as ciphertext glyphs, then cycle
//! through encrypted symbols (fast, then slow) before settling on the
//! original plaintext with a color gradient.

use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair, Gradient};

const TYPING_SPEED: usize = 1;
const FAST_FRAMES: usize = 20;
const FAST_DURATION: u32 = 2;
const SLOW_FRAMES: usize = 30;
const SLOW_DURATION_MIN: u32 = 2;
const SLOW_DURATION_MAX: u32 = 10;
const CIPHER_GRADIENT_STEPS: usize = 10;
const FINAL_GRADIENT_STEPS: usize = 12;
const DISCOVERED_STEPS: usize = 10;
const DISCOVERED_DURATION: u32 = 8;
const MAX_FRAMES: usize = 200_000;

const CIPHERTEXT_COLORS: [Color; 2] = [
    Color { r: 0x00, g: 0xd1, b: 0xff },
    Color { r: 0x00, g: 0x92, b: 0xcb },
];
const PLAINTEXT_COLORS: [Color; 2] = [
    Color { r: 0xed, g: 0xb2, b: 0x00 },
    Color { r: 0x95, g: 0x71, b: 0x00 },
];
const FINAL_GRADIENT_STOPS: [Color; 3] = [
    Color { r: 0x8a, g: 0x00, b: 0x8a },
    Color { r: 0x00, g: 0xd1, b: 0xff },
    Color { r: 0xff, g: 0xff, b: 0xff },
];

struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed | 1 }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn gen_index(&mut self, len: usize) -> usize {
        if len == 0 {
            0
        } else {
            (self.next_u64() as usize) % len
        }
    }

    fn choice<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.gen_index(items.len())]
    }

    /// Inclusive range, matching Python `random.randint(lo, hi)`.
    fn randint(&mut self, lo: u32, hi: u32) -> u32 {
        if hi <= lo {
            lo
        } else {
            lo + (self.next_u64() as u32) % (hi - lo + 1)
        }
    }
}

fn seed_from(input: &str) -> u64 {
    let mut h = 0xD3C2_9700_DEC2_4077u64;
    for &b in input.as_bytes() {
        h = h.wrapping_mul(0x0100_0000_01B3).wrapping_add(u64::from(b));
    }
    h
}

fn encrypted_symbols() -> Vec<String> {
    let mut symbols = Vec::with_capacity((127 - 32) + (352 - 231));
    for n in 32..127u32 {
        if let Some(ch) = char::from_u32(n) {
            symbols.push(ch.to_string());
        }
    }
    for n in 231..352u32 {
        if let Some(ch) = char::from_u32(n) {
            symbols.push(ch.to_string());
        }
    }
    symbols
}

struct AnimKey {
    symbol: String,
    color: Color,
    duration: u32,
}

struct CharAnim {
    id: CharacterId,
    keys: Vec<AnimKey>,
    idx: usize,
    shown: u32,
    active: bool,
}

fn set_appearance(term: &mut Terminal, id: CharacterId, symbol: &str, color: Color) {
    if let Some(ch) = term.get_character_mut(id) {
        ch.animation
            .set_appearance(symbol, Some(ColorPair::fg(color)));
    }
}

fn apply_key(term: &mut Terminal, id: CharacterId, key: &AnimKey) {
    set_appearance(term, id, &key.symbol, key.color);
}

fn vertical_progress(row: i32, text_bottom: i32, text_top: i32) -> f64 {
    if text_top == text_bottom {
        0.0
    } else {
        f64::from(row - text_bottom) / f64::from(text_top - text_bottom)
    }
}

fn pick_color(rng: &mut Rng, colors: &[Color], fallback: Color) -> Color {
    if colors.is_empty() {
        fallback
    } else {
        *rng.choice(colors)
    }
}

/// Movie-style decryption effect.
pub struct Decrypt;

impl Decrypt {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Decrypt {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Decrypt {
    fn name(&self) -> &str {
        "decrypt"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        term.hide_all();

        let symbols = encrypted_symbols();
        if symbols.is_empty() {
            return vec![term.render_frame()];
        }

        let mut rng = Rng::new(seed_from(input));
        let cipher_gradient = Gradient::new(&CIPHERTEXT_COLORS, CIPHER_GRADIENT_STEPS);
        let cipher_spectrum: Vec<Color> = cipher_gradient.spectrum().to_vec();
        let final_gradient = Gradient::new(&FINAL_GRADIENT_STOPS, FINAL_GRADIENT_STEPS);

        let infos: Vec<(CharacterId, String, i32)> = term
            .get_characters()
            .iter()
            .map(|ch| (ch.id, ch.input_symbol.clone(), ch.input_coord.row))
            .collect();

        if infos.is_empty() {
            return vec![term.render_frame()];
        }

        let text_bottom = infos.iter().map(|(_, _, row)| *row).min().unwrap_or(1);
        let text_top = infos.iter().map(|(_, _, row)| *row).max().unwrap_or(1);

        let mut anims: Vec<CharAnim> = Vec::with_capacity(infos.len());
        for (id, input_symbol, row) in &infos {
            let progress = vertical_progress(*row, text_bottom, text_top);
            let final_color = final_gradient
                .mapped_color(progress)
                .unwrap_or(FINAL_GRADIENT_STOPS[FINAL_GRADIENT_STOPS.len() - 1]);

            let mut keys = Vec::new();

            for _ in 0..FAST_FRAMES {
                keys.push(AnimKey {
                    symbol: rng.choice(&symbols).clone(),
                    color: pick_color(&mut rng, &cipher_spectrum, CIPHERTEXT_COLORS[0]),
                    duration: FAST_DURATION,
                });
            }

            for _ in 0..SLOW_FRAMES {
                keys.push(AnimKey {
                    symbol: rng.choice(&symbols).clone(),
                    color: pick_color(&mut rng, &cipher_spectrum, CIPHERTEXT_COLORS[0]),
                    duration: rng.randint(SLOW_DURATION_MIN, SLOW_DURATION_MAX),
                });
            }

            let discovered = Gradient::new(
                &[PLAINTEXT_COLORS[0], PLAINTEXT_COLORS[1], final_color],
                DISCOVERED_STEPS,
            );
            if discovered.is_empty() {
                keys.push(AnimKey {
                    symbol: input_symbol.clone(),
                    color: final_color,
                    duration: DISCOVERED_DURATION,
                });
            } else {
                for color in discovered.spectrum() {
                    keys.push(AnimKey {
                        symbol: input_symbol.clone(),
                        color: *color,
                        duration: DISCOVERED_DURATION,
                    });
                }
            }

            anims.push(CharAnim {
                id: *id,
                keys,
                idx: 0,
                shown: 0,
                active: false,
            });
        }

        let mut pending: Vec<CharacterId> = infos.iter().map(|(id, _, _)| *id).collect();
        let mut decrypting = false;
        let mut frames = Vec::new();

        loop {
            if !decrypting {
                if pending.is_empty() {
                    decrypting = true;
                    for anim in &mut anims {
                        if anim.keys.is_empty() {
                            anim.active = false;
                            continue;
                        }
                        anim.active = true;
                        anim.idx = 0;
                        anim.shown = 0;
                        apply_key(&mut term, anim.id, &anim.keys[0]);
                    }
                } else {
                    for _ in 0..TYPING_SPEED {
                        let Some(id) = pending.first().copied() else {
                            break;
                        };
                        pending.remove(0);
                        term.set_character_visibility(id, true);
                        let symbol = rng.choice(&symbols);
                        let color = pick_color(&mut rng, &CIPHERTEXT_COLORS, CIPHERTEXT_COLORS[0]);
                        set_appearance(&mut term, id, symbol, color);
                    }
                }
            }

            if decrypting {
                let mut updates: Vec<(CharacterId, usize)> = Vec::new();
                for anim in &mut anims {
                    if !anim.active {
                        continue;
                    }
                    anim.shown = anim.shown.saturating_add(1);
                    let duration = anim
                        .keys
                        .get(anim.idx)
                        .map(|key| key.duration)
                        .unwrap_or(1);
                    if anim.shown >= duration {
                        anim.idx += 1;
                        anim.shown = 0;
                        if anim.idx >= anim.keys.len() {
                            anim.active = false;
                        } else {
                            updates.push((anim.id, anim.idx));
                        }
                    }
                }
                for (id, idx) in updates {
                    if let Some(anim) = anims.iter().find(|anim| anim.id == id) {
                        if let Some(key) = anim.keys.get(idx) {
                            apply_key(&mut term, id, key);
                        }
                    }
                }
            }

            frames.push(term.render_frame());

            if decrypting && anims.iter().all(|anim| !anim.active) {
                break;
            }
            if frames.len() >= MAX_FRAMES {
                break;
            }
        }

        frames
    }
}
