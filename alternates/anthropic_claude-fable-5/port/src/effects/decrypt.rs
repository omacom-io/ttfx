//! Decrypt effect: characters are "typed" as encrypted symbols, then each
//! cycles through ciphertext before decrypting into the plaintext.
//!
//! Port of terminaltexteffects/effects/effect_decrypt.py.

use std::collections::VecDeque;

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Small deterministic xorshift64 PRNG (no external crates available).
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9e37_79b9_7f4a_7c15 } else { seed })
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Random integer in `lo..=hi`.
    fn range(&mut self, lo: u32, hi: u32) -> u32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next() % (hi - lo + 1) as u64) as u32
    }

    fn choice<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[(self.next() % items.len() as u64) as usize]
    }
}

/// Symbols used for the "encrypted" ciphertext, matching the Python original:
/// lowercase letters plus assorted ASCII punctuation ranges.
fn encrypted_symbols() -> Vec<char> {
    let mut symbols = Vec::new();
    for n in 97u8..123 {
        symbols.push(n as char);
    }
    for n in 33u8..48 {
        symbols.push(n as char);
    }
    for n in 58u8..65 {
        symbols.push(n as char);
    }
    for n in 91u8..97 {
        symbols.push(n as char);
    }
    for n in 123u8..127 {
        symbols.push(n as char);
    }
    symbols
}

pub struct Decrypt;

impl Decrypt {
    pub fn new() -> Self {
        Decrypt
    }
}

impl Default for Decrypt {
    fn default() -> Self {
        Decrypt::new()
    }
}

impl Effect for Decrypt {
    fn name(&self) -> &str {
        "decrypt"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());

        // Seed deterministically from the input so runs are repeatable.
        let seed = input
            .bytes()
            .fold(0xDEC0_DE00_5EED_1234u64, |acc, b| {
                acc.rotate_left(5) ^ b as u64
            });
        let mut rng = Rng::new(seed);

        let symbols = encrypted_symbols();
        let ciphertext_colors = [
            Color::from_hex("008000").expect("valid hex"),
            Color::from_hex("00cb00").expect("valid hex"),
            Color::from_hex("00ff00").expect("valid hex"),
        ];
        let final_color = Color::from_hex("eda000").expect("valid hex");
        let white = Color::new(255, 255, 255);
        // White -> final color "discovered" gradient (Python: Gradient(white, final, steps=10)).
        let discovered_gradient = Gradient::new(&[white, final_color], 10);

        // Build the scenes for every character up front, as the Python
        // build() does.
        for character in terminal.get_characters_mut() {
            let input_symbol = character.input_symbol;

            // Typing scene: blocks fade in, then a random encrypted symbol.
            let typing_color = *rng.choice(&ciphertext_colors);
            {
                let typing_scn = character.animation.new_scene("typing", false);
                for block in ['▉', '▓', '▒', '░'] {
                    typing_scn.add_frame(block, 2, ColorPair::fg(typing_color), false);
                }
                let cipher_symbol = *rng.choice(&symbols);
                typing_scn.add_frame(cipher_symbol, 2, ColorPair::fg(typing_color), false);
            }

            // Decrypt scene: cycle random ciphertext symbols, then reveal the
            // plaintext through the white -> final color gradient.
            {
                let decrypt_scn = character.animation.new_scene("decrypt", false);
                let cycle_count = rng.range(1, 15);
                for _ in 0..cycle_count {
                    let cipher_symbol = *rng.choice(&symbols);
                    let color = *rng.choice(&ciphertext_colors);
                    let duration = rng.range(3, 10);
                    decrypt_scn.add_frame(cipher_symbol, duration, ColorPair::fg(color), false);
                }
                for color in &discovered_gradient.spectrum {
                    decrypt_scn.add_frame(input_symbol, 3, ColorPair::fg(*color), false);
                }
                // Hold the final plaintext appearance.
                decrypt_scn.add_frame(input_symbol, 1, ColorPair::fg(final_color), false);
            }
        }

        let mut frames: Vec<String> = Vec::new();
        const MAX_FRAMES_PER_PHASE: usize = 20_000;

        // Phase 1: typing. One character is typed per tick, in input order.
        let mut pending: VecDeque<usize> = terminal
            .get_characters()
            .iter()
            .map(|c| c.character_id)
            .collect();

        loop {
            if let Some(id) = pending.pop_front() {
                terminal.set_character_visibility(id, true);
                if let Some(character) = terminal
                    .get_characters_mut()
                    .iter_mut()
                    .find(|c| c.character_id == id)
                {
                    character.animation.activate_scene("typing");
                }
            }
            let active = terminal.tick();
            frames.push(terminal.get_formatted_output_string());
            if pending.is_empty() && active == 0 {
                break;
            }
            if frames.len() >= MAX_FRAMES_PER_PHASE {
                break;
            }
        }

        // Phase 2: decryption. Every character decrypts simultaneously.
        for character in terminal.get_characters_mut() {
            character.animation.activate_scene("decrypt");
        }

        let phase_start = frames.len();
        loop {
            let active = terminal.tick();
            frames.push(terminal.get_formatted_output_string());
            if active == 0 {
                break;
            }
            if frames.len() - phase_start >= MAX_FRAMES_PER_PHASE {
                break;
            }
        }

        frames
    }
}
