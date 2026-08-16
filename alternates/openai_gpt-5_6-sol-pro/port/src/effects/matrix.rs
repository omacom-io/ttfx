use std::collections::BTreeMap;

use super::Effect;
use crate::engine::{CharacterId, Terminal};
use crate::utils::{Color, Coord, Style};

const MATRIX_SYMBOLS: &[u8] =
    b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ@#$%&*+-=<>";

#[derive(Debug, Clone)]
pub struct Matrix;

impl Matrix {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Matrix {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Matrix {
    fn name(&self) -> &str {
        "matrix"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);
        let width = terminal.canvas().width();
        let height = terminal.canvas().height();
        let mut rng = MatrixRng::new(seed_from_input(input));

        let mut input_characters = BTreeMap::new();
        for character in terminal.characters_mut() {
            input_characters.insert(character.position, character.id);
            character.visible = false;
        }

        let mut rain_characters = vec![CharacterId(0); width * height];
        let mut rain_symbols = vec!['0'; width * height];

        for x in 0..width {
            for y in 0..height {
                let index = y * width + x;
                let symbol = random_matrix_symbol(&mut rng);
                let id = terminal.add_character(
                    symbol,
                    Coord::new(x as i32, y as i32),
                );

                if let Some(character) = terminal.character_mut(id) {
                    character.visible = false;
                    character.set_appearance(symbol, rain_style(0, 1));
                }

                rain_characters[index] = id;
                rain_symbols[index] = symbol;
            }
        }

        let mut columns = (0..width)
            .map(|_| RainColumn::new(height, &mut rng))
            .collect::<Vec<_>>();
        let mut frames = Vec::new();

        let rain_duration = (height.saturating_mul(4) + width / 2 + 24).clamp(32, 120);

        for _ in 0..rain_duration {
            for column in &mut columns {
                column.step(height, &mut rng);
            }

            for x in 0..width {
                let column = &columns[x];

                for y in 0..height {
                    let index = y * width + x;
                    let distance = column.head - y as i32;
                    let visible = distance >= 0 && distance < column.length as i32;

                    if visible && rng.chance(1, 12) {
                        rain_symbols[index] = random_matrix_symbol(&mut rng);
                    }

                    if let Some(character) = terminal.character_mut(rain_characters[index]) {
                        character.visible = visible;

                        if visible {
                            character.set_appearance(
                                rain_symbols[index],
                                rain_style(distance as usize, column.length),
                            );
                        }
                    }
                }
            }

            frames.push(terminal.render_frame());
        }

        // Fill the canvas with code before resolving it back into the input.
        for reveal_row in 0..height {
            for y in 0..=reveal_row {
                for x in 0..width {
                    let index = y * width + x;

                    if rng.chance(1, 8) {
                        rain_symbols[index] = random_matrix_symbol(&mut rng);
                    }

                    if let Some(character) = terminal.character_mut(rain_characters[index]) {
                        character.visible = true;
                        character.set_appearance(
                            rain_symbols[index],
                            rain_style(reveal_row - y + 1, height.max(1)),
                        );
                    }
                }
            }

            frames.push(terminal.render_frame());
        }

        let mut resolve_coords = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                resolve_coords.push(Coord::new(x as i32, y as i32));
            }
        }
        rng.shuffle(&mut resolve_coords);

        let resolve_steps = (width * height).clamp(12, 36);
        let resolve_batch = resolve_coords.len().div_ceil(resolve_steps).max(1);

        for coords in resolve_coords.chunks(resolve_batch) {
            for character in terminal.characters_mut() {
                if character.visible
                    && rain_characters.contains(&character.id)
                    && rng.chance(1, 10)
                {
                    let symbol = random_matrix_symbol(&mut rng);
                    character.set_appearance(symbol, character.style.clone());
                }
            }

            for coord in coords {
                let index = coord.y as usize * width + coord.x as usize;

                if let Some(character) = terminal.character_mut(rain_characters[index]) {
                    character.visible = false;
                }

                if let Some(input_id) = input_characters.get(coord).copied() {
                    let final_color = final_color_for_row(coord.y as usize, height);

                    if let Some(character) = terminal.character_mut(input_id) {
                        character.visible = true;
                        character.set_appearance(
                            character.input_symbol,
                            Style::default().with_foreground(mix_color(
                                Color::rgb(235, 255, 235),
                                final_color,
                                0.25,
                            )),
                        );
                    }
                }
            }

            frames.push(terminal.render_frame());
        }

        // Fade the newly resolved text from the bright rain head color to its
        // final green gradient.
        for fade_step in 1..=8 {
            let progress = fade_step as f64 / 8.0;

            for character in terminal.characters_mut() {
                if input_characters
                    .get(&character.position)
                    .is_some_and(|id| *id == character.id)
                {
                    let final_color =
                        final_color_for_row(character.position.y as usize, height);
                    character.visible = true;
                    character.set_appearance(
                        character.input_symbol,
                        Style::default().with_foreground(mix_color(
                            Color::rgb(235, 255, 235),
                            final_color,
                            progress,
                        )),
                    );
                }
            }

            frames.push(terminal.render_frame());
        }

        for id in rain_characters {
            if let Some(character) = terminal.character_mut(id) {
                character.visible = false;
            }
        }

        for character in terminal.characters_mut() {
            if input_characters
                .get(&character.position)
                .is_some_and(|id| *id == character.id)
            {
                character.visible = true;
                character.set_appearance(
                    character.input_symbol,
                    Style::default().with_foreground(final_color_for_row(
                        character.position.y as usize,
                        height,
                    )),
                );
            }
        }

        let final_frame = terminal.render_frame();
        if frames.last() != Some(&final_frame) {
            frames.push(final_frame);
        }

        frames
    }
}

#[derive(Debug, Clone)]
struct RainColumn {
    head: i32,
    length: usize,
    fall_delay: usize,
    delay_tick: usize,
    restart_delay: usize,
}

impl RainColumn {
    fn new(height: usize, rng: &mut MatrixRng) -> Self {
        let maximum_length = height.max(3);
        let minimum_length = maximum_length.min(3);

        Self {
            head: -(rng.range(height.saturating_add(1)) as i32),
            length: minimum_length
                + rng.range(maximum_length.saturating_sub(minimum_length) + 1),
            fall_delay: 1 + rng.range(4),
            delay_tick: 0,
            restart_delay: rng.range(10),
        }
    }

    fn step(&mut self, height: usize, rng: &mut MatrixRng) {
        if self.restart_delay > 0 {
            self.restart_delay -= 1;
            return;
        }

        self.delay_tick += 1;
        if self.delay_tick < self.fall_delay {
            return;
        }

        self.delay_tick = 0;
        self.head += 1;

        if self.head - self.length as i32 > height as i32 {
            let maximum_length = height.max(3);
            let minimum_length = maximum_length.min(3);

            self.head = -(rng.range(height.saturating_add(1)) as i32);
            self.length = minimum_length
                + rng.range(maximum_length.saturating_sub(minimum_length) + 1);
            self.fall_delay = 1 + rng.range(4);
            self.restart_delay = 2 + rng.range(14);
        }
    }
}

fn rain_style(distance_from_head: usize, tail_length: usize) -> Style {
    let progress = if tail_length <= 1 {
        0.0
    } else {
        distance_from_head as f64 / (tail_length - 1) as f64
    }
    .clamp(0.0, 1.0);

    let color = if distance_from_head == 0 {
        Color::rgb(225, 255, 225)
    } else {
        mix_color(
            Color::rgb(80, 255, 105),
            Color::rgb(0, 55, 12),
            progress,
        )
    };

    let mut style = Style::default().with_foreground(color);
    style.bold = distance_from_head <= 1;
    style.dim = progress > 0.72;
    style
}

fn final_color_for_row(row: usize, height: usize) -> Color {
    let progress = if height <= 1 {
        0.0
    } else {
        row as f64 / (height - 1) as f64
    };

    mix_color(
        Color::rgb(85, 255, 115),
        Color::rgb(0, 145, 45),
        progress,
    )
}

fn mix_color(start: Color, end: Color, progress: f64) -> Color {
    let progress = progress.clamp(0.0, 1.0);

    match (start, end) {
        (
            Color::Rgb {
                r: start_r,
                g: start_g,
                b: start_b,
            },
            Color::Rgb {
                r: end_r,
                g: end_g,
                b: end_b,
            },
        ) => Color::rgb(
            interpolate_channel(start_r, end_r, progress),
            interpolate_channel(start_g, end_g, progress),
            interpolate_channel(start_b, end_b, progress),
        ),
        (_, color) => color,
    }
}

fn interpolate_channel(start: u8, end: u8, progress: f64) -> u8 {
    (start as f64 + (end as f64 - start as f64) * progress)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn random_matrix_symbol(rng: &mut MatrixRng) -> char {
    MATRIX_SYMBOLS[rng.range(MATRIX_SYMBOLS.len())] as char
}

fn seed_from_input(input: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;

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

#[derive(Debug, Clone)]
struct MatrixRng {
    state: u64,
}

impl MatrixRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9e37_79b9_7f4a_7c15
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

    fn range(&mut self, upper_bound: usize) -> usize {
        if upper_bound <= 1 {
            0
        } else {
            (self.next_u64() % upper_bound as u64) as usize
        }
    }

    fn chance(&mut self, numerator: usize, denominator: usize) -> bool {
        denominator != 0 && self.range(denominator) < numerator
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for index in (1..values.len()).rev() {
            let other = self.range(index + 1);
            values.swap(index, other);
        }
    }
}
