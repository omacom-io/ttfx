
use super::Effect;
use crate::engine::terminal::Terminal;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Style};

#[derive(Debug, Clone, Copy, Default)]
pub struct Beams;

impl Beams {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Beams {
    fn name(&self) -> &str {
        "beams"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);
        let width = terminal.canvas().width();
        let height = terminal.canvas().height();
        let original_character_count = terminal.characters().len();

        for character in terminal
            .characters_mut()
            .iter_mut()
            .take(original_character_count)
        {
            character.visible = false;
        }

        let mut rng = SimpleRng::new(hash_input(input));
        let mut beams = Vec::with_capacity(width.saturating_add(height));

        for y in 0..height {
            let direction = if rng.next_bool() { 1 } else { -1 };
            let position = if direction > 0 {
                -1.0
            } else {
                width as f64
            };

            beams.push(Beam {
                orientation: Orientation::Horizontal,
                fixed_coordinate: y as i32,
                position,
                direction,
                speed: rng.range_f64(0.8, 1.8),
                start_tick: 0,
                finished: false,
            });
        }

        for x in 0..width {
            let direction = if rng.next_bool() { 1 } else { -1 };
            let position = if direction > 0 {
                -1.0
            } else {
                height as f64
            };

            beams.push(Beam {
                orientation: Orientation::Vertical,
                fixed_coordinate: x as i32,
                position,
                direction,
                speed: rng.range_f64(0.55, 1.25),
                start_tick: 0,
                finished: false,
            });
        }

        shuffle(&mut beams, &mut rng);

        for (index, beam) in beams.iter_mut().enumerate() {
            beam.start_tick = index.saturating_mul(2);
        }

        let mut illuminated_ages = vec![None::<u32>; original_character_count];
        let mut frames = Vec::new();
        let mut tick = 0usize;
        let mut beams_finished_at = None;
        let maximum_beam_ticks = beams
            .len()
            .saturating_mul(2)
            .saturating_add(width.max(height).saturating_mul(4))
            .saturating_add(32);

        while tick < maximum_beam_ticks {
            for age in illuminated_ages.iter_mut().flatten() {
                *age = age.saturating_add(1);
            }

            for beam in &mut beams {
                if beam.finished || tick < beam.start_tick {
                    continue;
                }

                let previous_position = beam.position;
                beam.position += beam.speed * f64::from(beam.direction);

                for (index, character) in terminal
                    .characters()
                    .iter()
                    .take(original_character_count)
                    .enumerate()
                {
                    let is_crossed = match beam.orientation {
                        Orientation::Horizontal => {
                            character.position.y == beam.fixed_coordinate
                                && crossed(
                                    previous_position,
                                    beam.position,
                                    f64::from(character.position.x),
                                )
                        }
                        Orientation::Vertical => {
                            character.position.x == beam.fixed_coordinate
                                && crossed(
                                    previous_position,
                                    beam.position,
                                    f64::from(character.position.y),
                                )
                        }
                    };

                    if is_crossed && illuminated_ages[index].is_none() {
                        illuminated_ages[index] = Some(0);
                    }
                }

                let limit = match beam.orientation {
                    Orientation::Horizontal => width as f64,
                    Orientation::Vertical => height as f64,
                };

                beam.finished = if beam.direction > 0 {
                    beam.position > limit
                } else {
                    beam.position < -1.0
                };
            }

            apply_illumination(
                &mut terminal,
                original_character_count,
                &illuminated_ages,
            );

            let temporary_beams =
                add_visible_beams(&mut terminal, &beams, tick, width, height);
            frames.push(terminal.render_frame());

            for id in temporary_beams.into_iter().rev() {
                terminal.remove_character(id);
            }

            if beams.iter().all(|beam| beam.finished) {
                let finished_tick = *beams_finished_at.get_or_insert(tick);
                if tick.saturating_sub(finished_tick) >= 8 {
                    break;
                }
            }

            tick = tick.saturating_add(1);
        }

        for (index, character) in terminal
            .characters_mut()
            .iter_mut()
            .take(original_character_count)
            .enumerate()
        {
            character.visible = true;
            illuminated_ages[index] = Some(8);
        }
        apply_illumination(
            &mut terminal,
            original_character_count,
            &illuminated_ages,
        );

        let maximum_diagonal = width
            .saturating_sub(1)
            .saturating_add(height.saturating_sub(1));

        for diagonal in 0..=maximum_diagonal {
            for character in terminal
                .characters_mut()
                .iter_mut()
                .take(original_character_count)
            {
                let coordinate_sum =
                    character.position.x.max(0) as usize + character.position.y.max(0) as usize;

                if coordinate_sum <= diagonal {
                    let progress = if height <= 1 {
                        1.0
                    } else {
                        character.position.y.max(0) as f64 / (height - 1) as f64
                    };

                    character.set_appearance(
                        character.input_symbol,
                        Style::default().with_foreground(final_gradient(progress)),
                    );
                    character.visible = true;
                }
            }

            frames.push(terminal.render_frame());
        }

        if frames.is_empty() {
            frames.push(terminal.render_frame());
        }

        frames
    }
}

#[derive(Debug, Clone, Copy)]
enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone)]
struct Beam {
    orientation: Orientation,
    fixed_coordinate: i32,
    position: f64,
    direction: i32,
    speed: f64,
    start_tick: usize,
    finished: bool,
}

fn crossed(start: f64, end: f64, target: f64) -> bool {
    let lower = start.min(end);
    let upper = start.max(end);
    target >= lower && target <= upper
}

fn apply_illumination(
    terminal: &mut Terminal,
    original_character_count: usize,
    illuminated_ages: &[Option<u32>],
) {
    for (index, character) in terminal
        .characters_mut()
        .iter_mut()
        .take(original_character_count)
        .enumerate()
    {
        let Some(age) = illuminated_ages[index] else {
            character.visible = false;
            continue;
        };

        character.visible = true;
        let progress = (age as f64 / 8.0).clamp(0.0, 1.0);
        character.set_appearance(
            character.input_symbol,
            Style::default().with_foreground(beam_gradient(progress)),
        );
    }
}

fn add_visible_beams(
    terminal: &mut Terminal,
    beams: &[Beam],
    tick: usize,
    width: usize,
    height: usize,
) -> Vec<crate::engine::character::CharacterId> {
    const ROW_SYMBOLS: [char; 3] = ['▂', '▁', '_'];
    const COLUMN_SYMBOLS: [char; 4] = ['▌', '▍', '▎', '▏'];

    let mut ids = Vec::new();

    for beam in beams {
        if beam.finished || tick < beam.start_tick {
            continue;
        }

        let moving_coordinate = beam.position.round() as i32;
        let (coord, symbol) = match beam.orientation {
            Orientation::Horizontal => (
                Coord::new(moving_coordinate, beam.fixed_coordinate),
                ROW_SYMBOLS[tick % ROW_SYMBOLS.len()],
            ),
            Orientation::Vertical => (
                Coord::new(beam.fixed_coordinate, moving_coordinate),
                COLUMN_SYMBOLS[tick % COLUMN_SYMBOLS.len()],
            ),
        };

        if coord.x < 0
            || coord.y < 0
            || coord.x as usize >= width
            || coord.y as usize >= height
        {
            continue;
        }

        let id = terminal.add_character(symbol, coord);
        if let Some(character) = terminal.character_mut(id) {
            character.set_appearance(
                symbol,
                Style::default()
                    .with_foreground(Color::rgb(255, 255, 255)),
            );
        }
        ids.push(id);
    }

    ids
}

fn beam_gradient(progress: f64) -> Color {
    three_stop_gradient(
        Color::rgb(255, 255, 255),
        Color::rgb(0, 209, 255),
        Color::rgb(138, 0, 138),
        progress,
    )
}

fn final_gradient(progress: f64) -> Color {
    three_stop_gradient(
        Color::rgb(138, 0, 138),
        Color::rgb(0, 209, 255),
        Color::rgb(255, 255, 255),
        progress,
    )
}

fn three_stop_gradient(first: Color, middle: Color, last: Color, progress: f64) -> Color {
    let progress = progress.clamp(0.0, 1.0);

    if progress <= 0.5 {
        interpolate_color(first, middle, progress * 2.0)
    } else {
        interpolate_color(middle, last, (progress - 0.5) * 2.0)
    }
}

fn interpolate_color(start: Color, end: Color, progress: f64) -> Color {
    let (start_r, start_g, start_b) = rgb_components(start);
    let (end_r, end_g, end_b) = rgb_components(end);
    let progress = progress.clamp(0.0, 1.0);

    let interpolate = |start: u8, end: u8| {
        (f64::from(start) + (f64::from(end) - f64::from(start)) * progress)
            .round()
            .clamp(0.0, 255.0) as u8
    };

    Color::rgb(
        interpolate(start_r, end_r),
        interpolate(start_g, end_g),
        interpolate(start_b, end_b),
    )
}

fn rgb_components(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb { r, g, b } => (r, g, b),
        Color::Ansi(value) => (value, value, value),
    }
}

fn hash_input(input: &str) -> u64 {
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

fn shuffle<T>(values: &mut [T], rng: &mut SimpleRng) {
    for upper in (1..values.len()).rev() {
        let index = rng.range_usize(upper + 1);
        values.swap(index, upper);
    }
}

#[derive(Debug, Clone)]
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
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

    fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 0
    }

    fn next_f64(&mut self) -> f64 {
        let value = self.next_u64() >> 11;
        value as f64 / ((1_u64 << 53) as f64)
    }

    fn range_f64(&mut self, minimum: f64, maximum: f64) -> f64 {
        minimum + (maximum - minimum) * self.next_f64()
    }

    fn range_usize(&mut self, upper_exclusive: usize) -> usize {
        if upper_exclusive <= 1 {
            0
        } else {
            (self.next_u64() % upper_exclusive as u64) as usize
        }
    }
}
