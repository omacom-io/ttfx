//! Sweep effect: a sweep passes over the canvas revealing the text as dim
//! block-noise, then a reverse sweep resolves each column into the final
//! gradient-colored text. Port of terminaltexteffects/effects/effect_sweep.py
//! adapted to this engine's simplified scene/tick model.

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Symbols cycled while a character is in its unresolved "noise" state.
const NOISE_SYMBOLS: [char; 4] = ['░', '▒', '▓', '▒'];

/// Ticks to hold between the end of the first sweep and the start of the second.
const SWEEP_GAP_TICKS: i32 = 10;

pub struct Sweep;

impl Sweep {
    pub fn new() -> Self {
        Sweep
    }
}

impl Effect for Sweep {
    fn name(&self) -> &str {
        "sweep"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;

        // Colors (mirroring the upstream defaults).
        let noise_color = Color::from_hex("474747").unwrap_or(Color::new(0x47, 0x47, 0x47));
        let white = Color::new(0xFF, 0xFF, 0xFF);
        let stops = [
            Color::from_hex("8A008A").unwrap_or(Color::new(0x8A, 0x00, 0x8A)),
            Color::from_hex("00D1FF").unwrap_or(Color::new(0x00, 0xD1, 0xFF)),
            white,
        ];
        let final_gradient = Gradient::new(&stops, 12);

        // Build the per-character scenes.
        for character in terminal.get_characters_mut() {
            let symbol = character.input_symbol;
            let row = character.input_coord.row;
            let fraction = if height > 1 {
                (row - 1) as f64 / (height - 1) as f64
            } else {
                0.0
            };
            let final_color = final_gradient
                .get_color_at_fraction(fraction)
                .unwrap_or(white);

            // First sweep: looping dim noise.
            {
                let noise_scn = character.animation.new_scene("noise", true);
                for &noise_symbol in NOISE_SYMBOLS.iter() {
                    noise_scn.add_frame(noise_symbol, 3, ColorPair::fg(noise_color), false);
                }
            }

            // Second sweep: flash bright blocks, then fade the input symbol
            // from white down to its final gradient color.
            {
                let resolve_scn = character.animation.new_scene("resolve", false);
                resolve_scn.add_frame('▓', 2, ColorPair::fg(white), false);
                resolve_scn.add_frame('▒', 2, ColorPair::fg(white), false);
                resolve_scn.add_frame('░', 2, ColorPair::fg(white), false);
                let fade = Gradient::new(&[white, final_color], 8);
                for color in fade.spectrum.iter() {
                    resolve_scn.add_frame(symbol, 2, ColorPair::fg(*color), false);
                }
            }
        }

        // Sweep schedule:
        //   ticks [0, width)                       first sweep, left -> right
        //   ticks [width, second_start)            hold
        //   ticks [second_start, second_start+width) second sweep, right -> left
        let second_start = width + SWEEP_GAP_TICKS;
        let second_end = second_start + width;
        let max_ticks = second_end + 200;

        let mut frames: Vec<String> = Vec::new();
        let mut tick: i32 = 0;

        loop {
            if tick < width {
                // First sweep reveals column `tick + 1`.
                let column = tick + 1;
                for character in terminal.get_characters_mut() {
                    if character.input_coord.column == column {
                        character.is_visible = true;
                        character.animation.activate_scene("noise");
                    }
                }
            } else if tick >= second_start && tick < second_end {
                // Second sweep resolves columns right-to-left.
                let column = width - (tick - second_start);
                for character in terminal.get_characters_mut() {
                    if character.input_coord.column == column {
                        character.is_visible = true;
                        character.animation.activate_scene("resolve");
                    }
                }
            }

            let active = terminal.tick();
            frames.push(terminal.get_formatted_output_string());

            tick += 1;
            if tick >= second_end && active == 0 {
                break;
            }
            if tick >= max_ticks {
                break;
            }
        }

        frames
    }
}
