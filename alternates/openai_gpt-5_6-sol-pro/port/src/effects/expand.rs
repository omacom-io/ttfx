use super::Effect;
use crate::engine::{Path, Terminal, Waypoint};
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Style};

const MOVEMENT_SPEED: f64 = 0.35;
const FINAL_GRADIENT_STEPS: usize = 12;
const EXPAND_GRADIENT_STEPS: usize = 10;

const FINAL_GRADIENT: [Color; 3] = [
    Color::rgb(0x8a, 0x00, 0x8a),
    Color::rgb(0x00, 0xd1, 0xff),
    Color::rgb(0xff, 0xff, 0xff),
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Expand;

impl Expand {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Expand {
    fn name(&self) -> &str {
        "expand"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);

        if terminal.characters().is_empty() {
            return Vec::new();
        }

        let width = terminal.canvas().width();
        let height = terminal.canvas().height();
        let center = Coord::new(
            (width.saturating_sub(1) / 2) as i32,
            (height.saturating_sub(1) / 2) as i32,
        );

        let targets: Vec<Coord> = terminal
            .characters()
            .iter()
            .map(|character| character.position)
            .collect();

        let distances: Vec<f64> = targets
            .iter()
            .map(|target| center.distance(*target))
            .collect();

        let final_colors: Vec<Color> = targets
            .iter()
            .map(|target| final_color_for_row(target.y, height))
            .collect();

        for character in terminal.characters_mut() {
            character.set_position(center);
            character.set_style(Style::default().with_foreground(Color::rgb(
                0xff, 0xff, 0xff,
            )));
        }

        for (character, target) in terminal
            .characters_mut()
            .iter_mut()
            .zip(targets.iter().copied())
        {
            let mut path = Path::with_waypoints(
                vec![Waypoint::new(center), Waypoint::new(target)],
                MOVEMENT_SPEED,
            );
            path.set_easing(in_out_quart);
            character.motion.activate_path(path);
        }

        let mut frames = Vec::new();
        let mut elapsed_steps = 0usize;

        loop {
            let has_active_character = terminal.characters().iter().any(|character| {
                character
                    .motion
                    .active_path()
                    .map(|path| path.is_active())
                    .unwrap_or(false)
            });

            if !has_active_character {
                break;
            }

            terminal.step();
            elapsed_steps += 1;

            for (index, character) in terminal.characters_mut().iter_mut().enumerate() {
                let distance = distances[index];
                let progress = if distance <= f64::EPSILON {
                    1.0
                } else {
                    let raw =
                        (elapsed_steps as f64 * MOVEMENT_SPEED / distance).clamp(0.0, 1.0);
                    in_out_quart(raw)
                };

                let color = stepped_color(
                    Color::rgb(0xff, 0xff, 0xff),
                    final_colors[index],
                    progress,
                    EXPAND_GRADIENT_STEPS,
                );

                character.set_style(Style::default().with_foreground(color));
            }

            frames.push(terminal.render_frame());
        }

        frames
    }
}

fn in_out_quart(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);

    if progress < 0.5 {
        8.0 * progress.powi(4)
    } else {
        1.0 - (-2.0 * progress + 2.0).powi(4) / 2.0
    }
}

fn final_color_for_row(row: i32, height: usize) -> Color {
    if height <= 1 {
        return FINAL_GRADIENT[0];
    }

    // The original effect's vertical gradient runs from the first stop at
    // the bottom of the canvas to the last stop at the top.
    let vertical_progress =
        1.0 - (row.max(0) as f64 / height.saturating_sub(1) as f64);

    multi_stop_color(
        &FINAL_GRADIENT,
        vertical_progress,
        FINAL_GRADIENT_STEPS,
    )
}

fn multi_stop_color(stops: &[Color], progress: f64, steps_per_segment: usize) -> Color {
    if stops.is_empty() {
        return Color::rgb(0xff, 0xff, 0xff);
    }

    if stops.len() == 1 {
        return stops[0];
    }

    let steps_per_segment = steps_per_segment.max(2);
    let segment_count = stops.len() - 1;
    let spectrum_len = segment_count * steps_per_segment;
    let index = rounded_index(progress, spectrum_len.saturating_sub(1));

    let segment = (index / steps_per_segment).min(segment_count - 1);
    let segment_index = index.saturating_sub(segment * steps_per_segment);
    let segment_progress = segment_index as f64 / (steps_per_segment - 1) as f64;

    interpolate_color(stops[segment], stops[segment + 1], segment_progress)
}

fn stepped_color(start: Color, end: Color, progress: f64, steps: usize) -> Color {
    let steps = steps.max(2);
    let index = rounded_index(progress, steps - 1);
    let quantized_progress = index as f64 / (steps - 1) as f64;
    interpolate_color(start, end, quantized_progress)
}

fn rounded_index(progress: f64, final_index: usize) -> usize {
    let value = progress.clamp(0.0, 1.0) * final_index as f64;
    let lower = value.floor();
    let fraction = value - lower;

    let rounded = if fraction < 0.5 {
        lower
    } else if fraction > 0.5 {
        lower + 1.0
    } else if lower as usize % 2 == 0 {
        lower
    } else {
        lower + 1.0
    };

    (rounded as usize).min(final_index)
}

fn interpolate_color(start: Color, end: Color, progress: f64) -> Color {
    let (start_r, start_g, start_b) = rgb_components(start);
    let (end_r, end_g, end_b) = rgb_components(end);
    let progress = progress.clamp(0.0, 1.0);

    Color::rgb(
        interpolate_channel(start_r, end_r, progress),
        interpolate_channel(start_g, end_g, progress),
        interpolate_channel(start_b, end_b, progress),
    )
}

fn rgb_components(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb { r, g, b } => (r, g, b),
        Color::Ansi(value) => (value, value, value),
    }
}

fn interpolate_channel(start: u8, end: u8, progress: f64) -> u8 {
    let value = start as f64 + (end as f64 - start as f64) * progress;
    value.round().clamp(0.0, 255.0) as u8
}
