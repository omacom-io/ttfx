use super::Effect;
use crate::engine::Terminal;
use crate::utils::easing::{in_out_sine, out_expo};
use crate::utils::{Color, Coord, Style};

const UNSTABLE_COLOR: Color = Color::rgb(255, 255, 255);
const UNSTABLE_MAGENTA: Color = Color::rgb(255, 0, 170);
const UNSTABLE_CYAN: Color = Color::rgb(0, 209, 255);

const FINAL_START: Color = Color::rgb(138, 0, 138);
const FINAL_MIDDLE: Color = Color::rgb(0, 209, 255);
const FINAL_END: Color = Color::rgb(255, 255, 255);

#[derive(Debug, Clone, Copy, Default)]
pub struct Unstable;

impl Unstable {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Unstable {
    fn name(&self) -> &str {
        "unstable"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);

        if terminal.characters().is_empty() {
            return vec![terminal.render_frame()];
        }

        let homes: Vec<Coord> = terminal
            .characters()
            .iter()
            .map(|character| character.position)
            .collect();

        let mut rng = SmallRng::from_input(input);
        let mut frames = Vec::new();

        // The text becomes progressively less stable, with characters shaking
        // independently and briefly flashing contrasting colors.
        for frame_index in 0..30 {
            let amplitude = match frame_index {
                0..=5 => 1,
                6..=13 => 2,
                14..=21 => 3,
                22..=25 => 2,
                _ => 1,
            };

            let width = terminal.canvas().width() as i32;
            let height = terminal.canvas().height() as i32;

            for (character, home) in terminal.characters_mut().iter_mut().zip(&homes) {
                let dx = rng.range_i32(-amplitude, amplitude);
                let dy = rng.range_i32(-amplitude, amplitude);
                let position = clamp_coord(home.offset(dx, dy), width, height);

                let color = match rng.next_u32() % 7 {
                    0 => UNSTABLE_MAGENTA,
                    1 => UNSTABLE_CYAN,
                    _ => UNSTABLE_COLOR,
                };

                character.set_position(position);
                character.set_appearance(
                    character.input_symbol,
                    Style::default().with_foreground(color),
                );
            }

            frames.push(terminal.render_frame());
        }

        let width = terminal.canvas().width() as i32;
        let height = terminal.canvas().height() as i32;
        let targets: Vec<Coord> = homes
            .iter()
            .map(|_| random_edge_coord(&mut rng, width, height))
            .collect();

        // Release the unstable characters toward random edges of the canvas.
        for step in 1..=18 {
            let progress = out_expo(step as f64 / 18.0);

            for ((character, home), target) in terminal
                .characters_mut()
                .iter_mut()
                .zip(&homes)
                .zip(&targets)
            {
                character.set_position(home.lerp(*target, progress));

                let color = if (step + character.id.0 as usize) % 4 == 0 {
                    UNSTABLE_CYAN
                } else {
                    UNSTABLE_COLOR
                };

                character.set_appearance(
                    character.input_symbol,
                    Style::default().with_foreground(color),
                );
            }

            frames.push(terminal.render_frame());
        }

        // Reassemble the text at its original coordinates.
        for step in 1..=24 {
            let progress = in_out_sine(step as f64 / 24.0);

            for ((character, target), home) in terminal
                .characters_mut()
                .iter_mut()
                .zip(&targets)
                .zip(&homes)
            {
                character.set_position(target.lerp(*home, progress));
                character.set_appearance(
                    character.input_symbol,
                    Style::default().with_foreground(UNSTABLE_COLOR),
                );
            }

            frames.push(terminal.render_frame());
        }

        // Finish with the stable vertical gradient used by the effect.
        let canvas_height = terminal.canvas().height();

        for (character, home) in terminal.characters_mut().iter_mut().zip(&homes) {
            let color = final_gradient(home.y, canvas_height);
            character.set_position(*home);
            character.set_appearance(
                character.input_symbol,
                Style::default().with_foreground(color),
            );
        }

        frames.push(terminal.render_frame());
        frames
    }
}

fn clamp_coord(coord: Coord, width: i32, height: i32) -> Coord {
    Coord::new(
        coord.x.clamp(0, width.saturating_sub(1)),
        coord.y.clamp(0, height.saturating_sub(1)),
    )
}

fn random_edge_coord(rng: &mut SmallRng, width: i32, height: i32) -> Coord {
    let max_x = width.saturating_sub(1);
    let max_y = height.saturating_sub(1);

    match rng.next_u32() % 4 {
        0 => Coord::new(0, rng.range_i32(0, max_y)),
        1 => Coord::new(max_x, rng.range_i32(0, max_y)),
        2 => Coord::new(rng.range_i32(0, max_x), 0),
        _ => Coord::new(rng.range_i32(0, max_x), max_y),
    }
}

fn final_gradient(row: i32, height: usize) -> Color {
    let progress = if height <= 1 {
        1.0
    } else {
        (row.max(0) as f64 / (height - 1) as f64).clamp(0.0, 1.0)
    };

    if progress <= 0.5 {
        interpolate_color(FINAL_START, FINAL_MIDDLE, progress * 2.0)
    } else {
        interpolate_color(FINAL_MIDDLE, FINAL_END, (progress - 0.5) * 2.0)
    }
}

fn interpolate_color(start: Color, end: Color, progress: f64) -> Color {
    let (
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
    ) = (start, end)
    else {
        return end;
    };

    let progress = progress.clamp(0.0, 1.0);
    let interpolate = |start: u8, end: u8| {
        (start as f64 + (end as f64 - start as f64) * progress)
            .round()
            .clamp(0.0, 255.0) as u8
    };

    Color::rgb(
        interpolate(start_r, end_r),
        interpolate(start_g, end_g),
        interpolate(start_b, end_b),
    )
}

#[derive(Debug, Clone, Copy)]
struct SmallRng {
    state: u64,
}

impl SmallRng {
    fn from_input(input: &str) -> Self {
        let mut state = 0xcbf2_9ce4_8422_2325_u64;

        for byte in input.bytes() {
            state ^= u64::from(byte);
            state = state.wrapping_mul(0x0000_0100_0000_01b3);
        }

        if state == 0 {
            state = 0x9e37_79b9_7f4a_7c15;
        }

        Self { state }
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        (value >> 32) as u32
    }

    fn range_i32(&mut self, minimum: i32, maximum: i32) -> i32 {
        if maximum <= minimum {
            return minimum;
        }

        let span = (i64::from(maximum) - i64::from(minimum) + 1) as u64;
        minimum + (u64::from(self.next_u32()) % span) as i32
    }
}
