//! Simplified Rust port of the TerminalTextEffects "errorcorrect" effect.
//!
//! In the full engine this effect corrupts the input with noisy placeholder
//! symbols and then progressively corrects each non-space character. The
//! visual beat here is kept intentionally simple: corrupt every visible glyph,
//! then reveal the original text in deterministic pseudo-random chunks.

use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::graphics::Color;
use super::Effect;

/// Deterministic LCG so the corruption order is stable between runs.
struct Lcg(u32);

impl Lcg {
    fn new(seed: u32) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        self.0
    }

    fn next_usize(&mut self, bound: usize) -> usize {
        if bound == 0 {
            0
        } else {
            (self.next_u32() as usize) % bound
        }
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        if items.len() < 2 {
            return;
        }

        for i in (1..items.len()).rev() {
            let j = (self.next_u32() as usize) % (i + 1);
            items.swap(i, j);
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Errorcorrect;

impl Errorcorrect {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Errorcorrect {
    fn name(&self) -> &str {
        "errorcorrect"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (width, height) = Terminal::autodetect_size();
        let mut terminal = Terminal::from_input(input, width, height);

        // Only visible glyphs are corrupted. Spaces remain blank so the noisy
        // frame stays readable and the effect does not fill the whole terminal.
        let originals: Vec<(u16, u16, String)> = terminal
            .characters
            .iter()
            .filter(|character| character.input_symbol != " ")
            .map(|character| {
                (
                    character.position.x as u16,
                    character.position.y as u16,
                    character.input_symbol.clone(),
                )
            })
            .collect();

        if originals.is_empty() {
            return vec![terminal.write_frame()];
        }

        let corrupt_symbols = [
            "#", "@", "!", "%", "&", "*", "?", "~", "$", "+", "=", "/", "\\", "<", ">",
        ];
        let mut rng = Lcg::new(0x5445_5846); // "TTXE"

        let mut error_style = CellStyle::new(Color::RED, Color::BLACK);
        error_style.bold = true;
        error_style.reverse = true;

        // Frame 0: all glyphs are corrupted.
        for (x, y, _) in &originals {
            let symbol = corrupt_symbols[rng.next_usize(corrupt_symbols.len())];
            terminal
                .canvas
                .set_cell(*x, *y, Cell::new(symbol, error_style));
        }

        let mut frames = vec![terminal.write_frame()];

        // Correct glyphs in deterministic pseudo-random order. Chunking keeps
        // the frame count reasonable for large inputs while still showing a
        // progressive correction.
        let mut order: Vec<usize> = (0..originals.len()).collect();
        rng.shuffle(&mut order);

        let target_frame_count = 22usize;
        let step = ((originals.len() as f32 / target_frame_count as f32).ceil() as usize).max(1);

        for chunk in order.chunks(step) {
            for &index in chunk {
                let (x, y, symbol) = &originals[index];
                terminal
                    .canvas
                    .set_cell(*x, *y, Cell::new(symbol.clone(), terminal.config.default_style));
            }
            frames.push(terminal.write_frame());
        }

        // Brief hold on the fully corrected result.
        for _ in 0..4 {
            frames.push(terminal.write_frame());
        }

        frames
    }
}
