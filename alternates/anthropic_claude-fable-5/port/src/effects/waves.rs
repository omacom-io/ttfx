//! Waves effect: a wave of block symbols sweeps across the text column by
//! column, leaving characters behind it fading into a final gradient color.
//!
//! Port of terminaltexteffects/effects/effect_waves.py. The Python version
//! chains a "wave" scene into a "final" scene via the event handler; this
//! port concatenates both into a single scene per character, which yields the
//! same visible sequence (wave symbols cycling `wave_count` times, then a fade
//! from the last wave color to the character's final gradient color).

use std::collections::VecDeque;

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Default wave symbols from the Python effect (rising then falling blocks).
const WAVE_SYMBOLS: [char; 15] = [
    '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█', '▇', '▆', '▅', '▄', '▃', '▂', '▁',
];

/// Number of times the wave symbol cycle repeats per character.
const WAVE_COUNT: usize = 7;

/// Ticks each wave frame is held (Python `wave_length`).
const WAVE_LENGTH: u32 = 2;

/// Interpolation steps for the wave gradient (Python `wave_gradient_steps`).
const WAVE_GRADIENT_STEPS: usize = 6;

/// Interpolation steps for the final gradient (Python `final_gradient_steps`).
const FINAL_GRADIENT_STEPS: usize = 12;

/// Steps in the per-character fade from wave color to final color.
const FADE_STEPS: usize = 10;

/// Ticks each fade frame is held.
const FADE_FRAME_DURATION: u32 = 3;

/// Safety cap so a pathological input can never loop forever.
const MAX_FRAMES: usize = 20_000;

pub struct Waves;

impl Waves {
    pub fn new() -> Self {
        Waves
    }
}

impl Effect for Waves {
    fn name(&self) -> &str {
        "waves"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let width = terminal.canvas.width;
        let height = terminal.canvas.height;

        // Wave gradient (Python default stops: f0ff65 -> ffb102 -> 31a0d4 -> ffb102 -> f0ff65).
        let wave_stops = [
            Color::from_hex("f0ff65").expect("valid hex"),
            Color::from_hex("ffb102").expect("valid hex"),
            Color::from_hex("31a0d4").expect("valid hex"),
            Color::from_hex("ffb102").expect("valid hex"),
            Color::from_hex("f0ff65").expect("valid hex"),
        ];
        let wave_gradient = Gradient::new(&wave_stops, WAVE_GRADIENT_STEPS);
        let wave_last_color = *wave_gradient
            .spectrum
            .last()
            .unwrap_or(&wave_stops[wave_stops.len() - 1]);

        // Final gradient (Python default stops: 833ab4 -> fd1d1d -> fcb045, diagonal direction).
        let final_stops = [
            Color::from_hex("833ab4").expect("valid hex"),
            Color::from_hex("fd1d1d").expect("valid hex"),
            Color::from_hex("fcb045").expect("valid hex"),
        ];
        let final_gradient = Gradient::new(&final_stops, FINAL_GRADIENT_STEPS);

        // Build one scene per character: wave cycles followed by the fade to the
        // character's final color (diagonal gradient mapping across the canvas).
        let diagonal_denominator =
            (width.saturating_sub(1) + height.saturating_sub(1)).max(1) as f64;
        for character in terminal.get_characters_mut() {
            let coord = character.input_coord;
            let fraction = ((coord.column - 1).max(0) as f64 + (coord.row - 1).max(0) as f64)
                / diagonal_denominator;
            let final_color = final_gradient
                .get_color_at_fraction(fraction)
                .unwrap_or(final_stops[final_stops.len() - 1]);

            let input_symbol = character.input_symbol;
            let scene = character.animation.new_scene("wave", false);

            // Wave frames: the wave gradient is spread across the symbol
            // sequence (Python apply_gradient_to_symbols).
            let symbol_count = WAVE_SYMBOLS.len();
            for _ in 0..WAVE_COUNT {
                for (index, symbol) in WAVE_SYMBOLS.iter().enumerate() {
                    let symbol_fraction = if symbol_count > 1 {
                        index as f64 / (symbol_count - 1) as f64
                    } else {
                        0.0
                    };
                    let color = wave_gradient
                        .get_color_at_fraction(symbol_fraction)
                        .unwrap_or(wave_last_color);
                    scene.add_frame(*symbol, WAVE_LENGTH, ColorPair::fg(color), false);
                }
            }

            // Fade frames: last wave color -> this character's final color.
            let fade_gradient = Gradient::new(&[wave_last_color, final_color], FADE_STEPS);
            for color in &fade_gradient.spectrum {
                scene.add_frame(input_symbol, FADE_FRAME_DURATION, ColorPair::fg(*color), false);
            }
        }

        // Group character ids into columns, left to right (Python
        // CharacterGroup.COLUMN_LEFT_TO_RIGHT).
        let mut column_indices: Vec<i32> = terminal
            .get_characters()
            .iter()
            .map(|c| c.input_coord.column)
            .collect();
        column_indices.sort_unstable();
        column_indices.dedup();

        let mut pending_columns: VecDeque<Vec<usize>> = column_indices
            .iter()
            .map(|column| {
                terminal
                    .get_characters()
                    .iter()
                    .filter(|c| c.input_coord.column == *column)
                    .map(|c| c.character_id)
                    .collect()
            })
            .collect();

        // Run loop: release one column per tick, then step everything.
        let mut frames: Vec<String> = Vec::new();
        let mut active_count = 0usize;

        while !pending_columns.is_empty() || active_count > 0 {
            if let Some(column) = pending_columns.pop_front() {
                for character_id in column {
                    terminal.set_character_visibility(character_id, true);
                    if let Some(character) = terminal
                        .characters
                        .iter_mut()
                        .find(|c| c.character_id == character_id)
                    {
                        character.animation.activate_scene("wave");
                    }
                }
            }

            active_count = terminal.tick();
            frames.push(terminal.get_formatted_output_string());

            if frames.len() >= MAX_FRAMES {
                break;
            }
        }

        // Hold the final resolved state for one extra frame.
        frames.push(terminal.get_formatted_output_string());
        frames
    }
}
