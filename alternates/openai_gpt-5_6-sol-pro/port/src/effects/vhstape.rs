
use super::Effect;
use crate::engine::Terminal;
use crate::utils::{Color, Coord, Style};

const STATIC_SYMBOLS: &[char] = &[
    '#', '%', '&', '*', '+', '-', '.', ':', ';', '=', '?', '@', '_', '|',
];

const RED: Color = Color::rgb(255, 45, 85);
const CYAN: Color = Color::rgb(0, 230, 255);
const BLUE: Color = Color::rgb(70, 90, 255);
const MAGENTA: Color = Color::rgb(255, 40, 220);
const WHITE: Color = Color::rgb(235, 245, 255);

#[derive(Debug, Clone, Copy, Default)]
pub struct Vhstape;

impl Vhstape {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Vhstape {
    fn name(&self) -> &str {
        "vhstape"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);

        if terminal.characters().is_empty() {
            return vec![terminal.render_frame()];
        }

        let width = terminal.canvas().width() as i32;
        let height = terminal.canvas().height();
        let originals = terminal
            .characters()
            .iter()
            .map(|character| {
                (
                    character.input_symbol,
                    character.position,
                    character.style.clone(),
                )
            })
            .collect::<Vec<_>>();

        let mut rng = TapeRng::new(hash_input(input));
        let mut frames = Vec::new();

        // Briefly establish the undistorted image before tracking noise begins.
        restore_characters(&mut terminal, &originals);
        frames.push(terminal.render_frame());

        for frame_index in 0..56usize {
            restore_characters(&mut terminal, &originals);

            let intensity = glitch_intensity(frame_index);
            let mut row_shifts = vec![0i32; height];

            if intensity > 0.0 {
                let band_count = 1 + rng.below(3);

                for _ in 0..band_count {
                    let start_row = rng.below(height);
                    let max_band_height = height.saturating_sub(start_row).min(3).max(1);
                    let band_height = 1 + rng.below(max_band_height);
                    let maximum_shift = if width >= 8 { 4 } else { 2 };
                    let mut shift = rng.range_i32(-maximum_shift, maximum_shift);

                    if shift == 0 {
                        shift = if rng.chance(0.5) { 1 } else { -1 };
                    }

                    for row in start_row..(start_row + band_height).min(height) {
                        row_shifts[row] = shift;
                    }
                }
            }

            for (character_index, character) in
                terminal.characters_mut().iter_mut().enumerate()
            {
                let (original_symbol, original_position, original_style) =
                    &originals[character_index];

                let row = original_position.y.max(0) as usize;
                let mut shift = row_shifts.get(row).copied().unwrap_or(0);

                // A rolling sinusoidal tracking wave bends a narrow part of the image.
                if intensity > 0.25 {
                    let wave = ((frame_index as f64 * 0.72)
                        + (original_position.y as f64 * 1.37))
                        .sin();

                    if wave > 0.72 {
                        shift += 1;
                    } else if wave < -0.72 {
                        shift -= 1;
                    }
                }

                let shifted_x = original_position.x + shift;
                character.position = Coord::new(shifted_x, original_position.y);
                character.visible = shifted_x >= 0 && shifted_x < width;

                let edge_of_tear = shift != 0
                    && (original_position.x == 0
                        || original_position.x == width.saturating_sub(1));

                let noise_probability =
                    (0.015 + intensity * 0.16 + if edge_of_tear { 0.12 } else { 0.0 })
                        .clamp(0.0, 0.45);

                let mut symbol = *original_symbol;
                if !original_symbol.is_whitespace() && rng.chance(noise_probability) {
                    symbol = STATIC_SYMBOLS[rng.below(STATIC_SYMBOLS.len())];
                }

                let mut style = original_style.clone();
                if !original_symbol.is_whitespace() {
                    let color_roll = rng.next_f64();

                    if shift > 0 && color_roll < 0.68 {
                        style = foreground_style(style, CYAN);
                    } else if shift < 0 && color_roll < 0.68 {
                        style = foreground_style(style, RED);
                    } else if rng.chance(intensity * 0.08) {
                        let color = match rng.below(4) {
                            0 => RED,
                            1 => CYAN,
                            2 => BLUE,
                            _ => MAGENTA,
                        };
                        style = foreground_style(style, color);
                    } else if frame_index >= 46 && frame_index < 55 {
                        style = foreground_style(style, WHITE);
                        style.dim = frame_index % 3 == 0;
                    }
                }

                character.set_appearance(symbol, style);
            }

            frames.push(terminal.render_frame());

            // Occasional duplicated frames emulate a tape briefly losing tracking.
            if intensity > 0.7 && rng.chance(0.12) {
                frames.push(frames.last().cloned().unwrap_or_default());
            }
        }

        // Always leave the text in its original position, symbols, and styling.
        restore_characters(&mut terminal, &originals);
        frames.push(terminal.render_frame());

        frames
    }
}

fn foreground_style(mut style: Style, color: Color) -> Style {
    style.colors.foreground = Some(color);
    style
}

fn restore_characters(
    terminal: &mut Terminal,
    originals: &[(char, Coord, Style)],
) {
    for (character, (symbol, position, style)) in
        terminal.characters_mut().iter_mut().zip(originals.iter())
    {
        character.position = *position;
        character.visible = true;
        character.set_appearance(*symbol, style.clone());
    }
}

fn glitch_intensity(frame: usize) -> f64 {
    match frame {
        0..=5 => frame as f64 / 12.0,
        6..=15 => 0.45,
        16..=37 => {
            let pulse = ((frame as f64 - 16.0) * 0.83).sin().abs();
            0.58 + pulse * 0.42
        }
        38..=45 => 0.48,
        46..=55 => (55usize.saturating_sub(frame)) as f64 / 18.0,
        _ => 0.0,
    }
}

fn hash_input(input: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;

    for byte in input.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    if hash == 0 {
        0x9e37_79b9_7f4a_7c15
    } else {
        hash
    }
}

#[derive(Debug, Clone, Copy)]
struct TapeRng {
    state: u64,
}

impl TapeRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0xa076_1d64_78bd_642f
            } else {
                seed
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }

    fn next_f64(&mut self) -> f64 {
        const SCALE: f64 = 1.0 / ((1u64 << 53) as f64);
        ((self.next_u64() >> 11) as f64) * SCALE
    }

    fn chance(&mut self, probability: f64) -> bool {
        self.next_f64() < probability.clamp(0.0, 1.0)
    }

    fn below(&mut self, upper: usize) -> usize {
        if upper <= 1 {
            0
        } else {
            (self.next_u64() % upper as u64) as usize
        }
    }

    fn range_i32(&mut self, minimum: i32, maximum: i32) -> i32 {
        if maximum <= minimum {
            return minimum;
        }

        let width = (maximum - minimum + 1) as usize;
        minimum + self.below(width) as i32
    }
}
