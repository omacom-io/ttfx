
use std::collections::HashSet;

use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::Terminal;
use crate::utils::easing::out_quad;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Style};

const RING_COLORS: [Color; 3] = [
    Color::rgb(171, 72, 255),
    Color::rgb(231, 178, 178),
    Color::rgb(255, 254, 189),
];

const ENTRANCE_FRAMES: usize = 28;
const SPIN_FRAMES: usize = 200;
const RETURN_FRAMES: usize = 32;
const ROTATION_TICKS_PER_POINT: usize = 4;

#[derive(Debug, Clone, Copy, Default)]
pub struct Rings;

impl Rings {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Rings {
    fn name(&self) -> &str {
        "rings"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);
        let width = terminal.canvas().width();
        let height = terminal.canvas().height();
        let center = Coord::new(
            (width.saturating_sub(1) / 2) as i32,
            (height.saturating_sub(1) / 2) as i32,
        );

        let mut characters: Vec<CharacterAssignment> = terminal
            .characters()
            .iter()
            .map(|character| CharacterAssignment {
                id: character.id,
                symbol: character.input_symbol,
                home: character.position,
                path: Vec::new(),
                path_index: 0,
                direction: 1,
                ring_color: RING_COLORS[0],
                final_color: final_color(character.position, width, height),
            })
            .collect();

        if characters.is_empty() {
            return vec![terminal.render_frame()];
        }

        let mut rng = LocalRng::from_text(input);
        shuffle(&mut characters, &mut rng);

        assign_rings(&mut characters, center);

        for assignment in &characters {
            if let Some(character) = terminal.character_mut(assignment.id) {
                character.visible = true;
                character.set_appearance(
                    assignment.symbol,
                    Style::default().with_foreground(assignment.final_color),
                );
            }
        }

        let mut frames = Vec::with_capacity(
            1 + ENTRANCE_FRAMES + SPIN_FRAMES + RETURN_FRAMES + 1,
        );

        frames.push(terminal.render_frame());

        for step in 1..=ENTRANCE_FRAMES {
            let progress = step as f64 / ENTRANCE_FRAMES as f64;

            for assignment in &characters {
                let target = assignment.path[assignment.path_index];
                let position = assignment.home.lerp(target, progress);
                let color = interpolate_color(
                    assignment.final_color,
                    assignment.ring_color,
                    progress,
                );

                if let Some(character) = terminal.character_mut(assignment.id) {
                    character.set_position(position);
                    character.set_appearance(
                        assignment.symbol,
                        Style::default().with_foreground(color),
                    );
                }
            }

            frames.push(terminal.render_frame());
        }

        for step in 0..SPIN_FRAMES {
            let shift = step / ROTATION_TICKS_PER_POINT;

            for assignment in &characters {
                let path_len = assignment.path.len();
                let offset = shift % path_len;
                let index = if assignment.direction > 0 {
                    (assignment.path_index + offset) % path_len
                } else {
                    (assignment.path_index + path_len - offset) % path_len
                };

                if let Some(character) = terminal.character_mut(assignment.id) {
                    character.set_position(assignment.path[index]);
                    character.set_appearance(
                        assignment.symbol,
                        Style::default().with_foreground(assignment.ring_color),
                    );
                }
            }

            frames.push(terminal.render_frame());
        }

        let return_starts: Vec<(CharacterId, Coord)> = characters
            .iter()
            .map(|assignment| {
                let position = terminal
                    .character(assignment.id)
                    .map(|character| character.position)
                    .unwrap_or(assignment.home);
                (assignment.id, position)
            })
            .collect();

        for step in 1..=RETURN_FRAMES {
            let raw_progress = step as f64 / RETURN_FRAMES as f64;
            let progress = out_quad(raw_progress);

            for (assignment, (_, start)) in characters.iter().zip(&return_starts) {
                let position = start.lerp(assignment.home, progress);
                let color = interpolate_color(
                    assignment.ring_color,
                    assignment.final_color,
                    progress,
                );

                if let Some(character) = terminal.character_mut(assignment.id) {
                    character.set_position(position);
                    character.set_appearance(
                        assignment.symbol,
                        Style::default().with_foreground(color),
                    );
                }
            }

            frames.push(terminal.render_frame());
        }

        for assignment in &characters {
            if let Some(character) = terminal.character_mut(assignment.id) {
                character.visible = true;
                character.set_position(assignment.home);
                character.set_appearance(
                    assignment.symbol,
                    Style::default().with_foreground(assignment.final_color),
                );
            }
        }

        frames.push(terminal.render_frame());
        frames
    }
}

#[derive(Debug, Clone)]
struct CharacterAssignment {
    id: CharacterId,
    symbol: char,
    home: Coord,
    path: Vec<Coord>,
    path_index: usize,
    direction: i32,
    ring_color: Color,
    final_color: Color,
}

fn assign_rings(characters: &mut [CharacterAssignment], center: Coord) {
    let mut character_index = 0;
    let mut radius = 1usize;
    let mut ring_index = 0usize;

    while character_index < characters.len() {
        let points = circle_points(center, radius as i32, radius.saturating_mul(7));

        if points.is_empty() {
            radius += 1;
            continue;
        }

        let count = points.len().min(characters.len() - character_index);
        let direction = if ring_index % 2 == 0 { 1 } else { -1 };
        let ring_color = RING_COLORS[ring_index % RING_COLORS.len()];

        for point_index in 0..count {
            let assignment = &mut characters[character_index + point_index];
            assignment.path = points.clone();
            assignment.path_index = point_index;
            assignment.direction = direction;
            assignment.ring_color = ring_color;
        }

        character_index += count;
        radius += 1;
        ring_index += 1;
    }
}

fn circle_points(center: Coord, radius: i32, count: usize) -> Vec<Coord> {
    let count = count.max(1);
    let mut points = Vec::with_capacity(count);
    let mut seen = HashSet::with_capacity(count);

    for index in 0..count {
        let angle = std::f64::consts::TAU * index as f64 / count as f64;
        let x = center.x + (angle.cos() * radius as f64).round() as i32;
        let y = center.y + (angle.sin() * radius as f64).round() as i32;
        let coord = Coord::new(x, y);

        if seen.insert(coord) {
            points.push(coord);
        }
    }

    if points.is_empty() {
        points.push(center);
    }

    points
}

fn final_color(coord: Coord, width: usize, height: usize) -> Color {
    let progress = if height > 1 {
        coord.y.max(0) as f64 / height.saturating_sub(1) as f64
    } else if width > 1 {
        coord.x.max(0) as f64 / width.saturating_sub(1) as f64
    } else {
        0.0
    };

    palette_color(&RING_COLORS, progress)
}

fn palette_color(colors: &[Color], progress: f64) -> Color {
    if colors.len() == 1 {
        return colors[0];
    }

    let scaled = progress.clamp(0.0, 1.0) * (colors.len() - 1) as f64;
    let lower = scaled.floor() as usize;
    let upper = (lower + 1).min(colors.len() - 1);
    let local_progress = scaled - lower as f64;

    interpolate_color(colors[lower], colors[upper], local_progress)
}

fn interpolate_color(start: Color, end: Color, progress: f64) -> Color {
    let (start_r, start_g, start_b) = color_components(start);
    let (end_r, end_g, end_b) = color_components(end);
    let progress = progress.clamp(0.0, 1.0);

    Color::rgb(
        interpolate_channel(start_r, end_r, progress),
        interpolate_channel(start_g, end_g, progress),
        interpolate_channel(start_b, end_b, progress),
    )
}

fn color_components(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb { r, g, b } => (r, g, b),
        Color::Ansi(value) => (value, value, value),
    }
}

fn interpolate_channel(start: u8, end: u8, progress: f64) -> u8 {
    let value = start as f64 + (end as f64 - start as f64) * progress;
    value.round().clamp(0.0, 255.0) as u8
}

fn shuffle<T>(values: &mut [T], rng: &mut LocalRng) {
    for index in (1..values.len()).rev() {
        let other = rng.index(index + 1);
        values.swap(index, other);
    }
}

#[derive(Debug, Clone)]
struct LocalRng {
    state: u64,
}

impl LocalRng {
    fn from_text(input: &str) -> Self {
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

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }

    fn index(&mut self, upper: usize) -> usize {
        if upper <= 1 {
            0
        } else {
            (self.next_u64() % upper as u64) as usize
        }
    }
}
