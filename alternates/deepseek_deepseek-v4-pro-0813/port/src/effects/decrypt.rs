
use super::Effect;
use crate::engine::canvas::Cell;
use crate::engine::character::EffectCharacter;
use crate::engine::terminal::Terminal;
use std::sync::atomic::{AtomicU64, Ordering};

/// A simple deterministic PRNG for reproducible reveal ordering.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        // LCG parameters from Numerical Recipes
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn next_range(&mut self, low: usize, high: usize) -> usize {
        assert!(low < high, "empty range");
        let span = (high - low) as f32;
        let val = self.next_f32() * span + low as f32;
        val.floor() as usize
    }
}

pub struct Decrypt {
    /// Total number of frames in the decryption animation.
    total_frames: usize,
    /// Seed for deterministic PRNG. Change to make effect differ between runs if desired.
    seed: u64,
}

impl Decrypt {
    pub fn new() -> Self {
        Self {
            total_frames: 30,
            seed: 0xDEAD_BEEF_CAFE_F00D,
        }
    }
}

impl Effect for Decrypt {
    fn name(&self) -> &str {
        "decrypt"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        // Determine terminal dimensions from input.
        let lines: Vec<&str> = input.lines().collect();
        let height = if lines.is_empty() {
            1
        } else {
            lines.len() as u16
        };
        let width = lines
            .iter()
            .map(|l| l.chars().count() as u16)
            .max()
            .unwrap_or(1)
            .max(1);

        let mut terminal = Terminal::from_input(input, width, height);

        // Map each character to a reveal frame index.
        let total_chars = terminal.characters.len();
        let mut reveal_order: Vec<usize> = (0..total_chars).collect();
        let mut rng = Rng::new(self.seed);

        // Shuffle reveal order using Fisher-Yates with our PRNG.
        for i in (1..total_chars).rev() {
            let j = rng.next_range(0, i + 1);
            reveal_order.swap(i, j);
        }

        // For each character, assign a reveal frame index based on its position in shuffled order.
        // The first character reveals at frame 0, the last at frame total_frames-1.
        let mut reveal_frames: Vec<usize> = vec![0; total_chars];
        for (idx, &char_idx) in reveal_order.iter().enumerate() {
            // Linear mapping: idx 0 -> 0, idx total_chars-1 -> total_frames-1
            let frame = (idx * (self.total_frames - 1)) / (total_chars - 1);
            reveal_frames[char_idx] = frame;
        }

        let mut frames = Vec::with_capacity(self.total_frames + 1);

        // Random symbol set for "ciphertext" appearance.
        let cipher_symbols: Vec<char> = "!@#$%^&*()_+-=<>?/\\|~[]{};:,.`'\"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
            .chars()
            .collect();

        // Generate frames.
        for frame_idx in 0..=self.total_frames {
            for (char_idx, character) in terminal.characters.iter_mut().enumerate() {
                let symbol = if reveal_frames[char_idx] <= frame_idx {
                    character.input_symbol.clone()
                } else {
                    // Use deterministic random choice based on char index + frame to avoid flicker.
                    let mut char_rng = Rng::new(self.seed ^ (char_idx as u64) ^ (frame_idx as u64));
                    let pick = char_rng.next_range(0, cipher_symbols.len());
                    cipher_symbols[pick].to_string()
                };

                // Update character output symbol.
                character.output_symbol = symbol.clone();

                // Update canvas cell.
                let pos = character.position;
                let x = pos.x as u16;
                let y = pos.y as u16;
                let style = character.style;
                terminal
                    .canvas
                    .set_cell(x, y, Cell::new(symbol, style));
            }

            // Render this frame.
            frames.push(terminal.write_frame());
        }

        frames
    }
}
