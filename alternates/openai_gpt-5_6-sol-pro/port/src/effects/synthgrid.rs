
use super::Effect;
use crate::engine::{CharacterId, Terminal};
use crate::utils::{Color, Coord, Style};

/// A retro synthwave grid that expands from the center before revealing the text.
#[derive(Debug, Clone, Copy, Default)]
pub struct Synthgrid;

impl Synthgrid {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Synthgrid {
    fn name(&self) -> &str {
        "synthgrid"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);
        let width = terminal.canvas().width();
        let height = terminal.canvas().height();

        let input_characters: Vec<(CharacterId, char, Coord)> = terminal
            .characters()
            .iter()
            .map(|character| (character.id, character.input_symbol, character.position))
            .collect();

        for (id, _, _) in &input_characters {
            if let Some(character) = terminal.character_mut(*id) {
                character.visible = false;
            }
        }

        let center = Coord::new(
            (width.saturating_sub(1) / 2) as i32,
            (height.saturating_sub(1) / 2) as i32,
        );

        let mut grid_characters = Vec::new();

        for y in 0..height {
            for x in 0..width {
                let coord = Coord::new(x as i32, y as i32);
                let vertical = (coord.x - center.x).rem_euclid(4) == 0;
                let horizontal = (coord.y - center.y).rem_euclid(2) == 0;

                if !vertical && !horizontal {
                    continue;
                }

                let symbol = match (vertical, horizontal) {
                    (true, true) => '┼',
                    (true, false) => '│',
                    (false, true) => '─',
                    (false, false) => unreachable!(),
                };

                let id = terminal.add_character(symbol, coord);
                let distance = normalized_distance(coord, center, width, height);
                let style = Style::default()
                    .with_foreground(two_stop_gradient(
                        (0xcc, 0x00, 0xcc),
                        (0xff, 0xff, 0xff),
                        distance,
                    ));

                if let Some(character) = terminal.character_mut(id) {
                    character.visible = false;
                    character.set_appearance(symbol, style);
                }

                grid_characters.push((id, coord));
            }
        }

        let mut frames = Vec::new();
        frames.push(terminal.render_frame());

        const EXPANSION_STEPS: usize = 10;

        for step in 0..EXPANSION_STEPS {
            let progress = (step + 1) as f64 / EXPANSION_STEPS as f64;

            for (id, coord) in &grid_characters {
                let distance = normalized_distance(*coord, center, width, height);

                if let Some(character) = terminal.character_mut(*id) {
                    character.visible = distance <= progress;
                }
            }

            frames.push(terminal.render_frame());
        }

        // Briefly hold the completed grid.
        frames.push(terminal.render_frame());
        frames.push(terminal.render_frame());

        let reveal_steps = (width + height).clamp(8, 24);
        let reveal_duration = reveal_steps + 4;

        for frame_index in 0..reveal_duration {
            for (id, coord) in &grid_characters {
                let disappearance_step =
                    deterministic_noise(coord.x, coord.y) % reveal_steps.max(1);

                if let Some(character) = terminal.character_mut(*id) {
                    character.visible = frame_index < disappearance_step;
                }
            }

            for (id, symbol, coord) in &input_characters {
                let delay = reveal_delay(*coord, width, height, reveal_steps);

                if frame_index < delay {
                    continue;
                }

                let stage = frame_index - delay;
                let final_style = Style::default()
                    .with_foreground(text_gradient(*coord, width, height));

                if let Some(character) = terminal.character_mut(*id) {
                    character.visible = true;

                    if symbol.is_whitespace() {
                        character.set_appearance(*symbol, final_style);
                    } else {
                        match stage {
                            0 => character.set_appearance(
                                '░',
                                Style::default()
                                    .with_foreground(Color::rgb(0x8a, 0x00, 0x8a)),
                            ),
                            1 => character.set_appearance(
                                '▒',
                                Style::default()
                                    .with_foreground(Color::rgb(0xcc, 0x00, 0xcc)),
                            ),
                            2 => character.set_appearance(
                                '▓',
                                Style::default()
                                    .with_foreground(Color::rgb(0x00, 0xd1, 0xff)),
                            ),
                            _ => character.set_appearance(*symbol, final_style),
                        }
                    }
                }
            }

            frames.push(terminal.render_frame());
        }

        for (id, _) in &grid_characters {
            if let Some(character) = terminal.character_mut(*id) {
                character.visible = false;
            }
        }

        for (id, symbol, coord) in &input_characters {
            let style =
                Style::default().with_foreground(text_gradient(*coord, width, height));

            if let Some(character) = terminal.character_mut(*id) {
                character.visible = true;
                character.set_appearance(*symbol, style);
            }
        }

        frames.push(terminal.render_frame());
        frames
    }
}

fn normalized_distance(coord: Coord, center: Coord, width: usize, height: usize) -> f64 {
    let dx = (coord.x - center.x).unsigned_abs() as f64;
    let dy = (coord.y - center.y).unsigned_abs() as f64;
    let max_dx = center.x.max(width as i32 - 1 - center.x).max(1) as f64;
    let max_dy = center.y.max(height as i32 - 1 - center.y).max(1) as f64;

    (dx / max_dx).max(dy / max_dy).clamp(0.0, 1.0)
}

fn reveal_delay(coord: Coord, width: usize, height: usize, steps: usize) -> usize {
    let denominator = width.saturating_sub(1) + height.saturating_sub(1);

    if denominator == 0 {
        return 0;
    }

    let diagonal = coord.x.max(0) as usize + coord.y.max(0) as usize;
    diagonal.saturating_mul(steps.saturating_sub(1)) / denominator
}

fn deterministic_noise(x: i32, y: i32) -> usize {
    let x = x as i64;
    let y = y as i64;
    let mixed = x
        .wrapping_mul(73_856_093)
        .wrapping_add(y.wrapping_mul(19_349_663))
        .wrapping_add((x ^ y).wrapping_mul(83_492_791));

    mixed.unsigned_abs() as usize
}

fn text_gradient(coord: Coord, width: usize, height: usize) -> Color {
    let denominator = width.saturating_sub(1) + height.saturating_sub(1);
    let progress = if denominator == 0 {
        1.0
    } else {
        let diagonal = coord.x.max(0) as usize + coord.y.max(0) as usize;
        diagonal as f64 / denominator as f64
    };

    three_stop_gradient(
        (0x8a, 0x00, 0x8a),
        (0x00, 0xd1, 0xff),
        (0xff, 0xff, 0xff),
        progress,
    )
}

fn two_stop_gradient(start: (u8, u8, u8), end: (u8, u8, u8), progress: f64) -> Color {
    let progress = progress.clamp(0.0, 1.0);

    Color::rgb(
        interpolate_channel(start.0, end.0, progress),
        interpolate_channel(start.1, end.1, progress),
        interpolate_channel(start.2, end.2, progress),
    )
}

fn three_stop_gradient(
    start: (u8, u8, u8),
    middle: (u8, u8, u8),
    end: (u8, u8, u8),
    progress: f64,
) -> Color {
    let progress = progress.clamp(0.0, 1.0);

    if progress <= 0.5 {
        two_stop_gradient(start, middle, progress * 2.0)
    } else {
        two_stop_gradient(middle, end, (progress - 0.5) * 2.0)
    }
}

fn interpolate_channel(start: u8, end: u8, progress: f64) -> u8 {
    let value = start as f64 + (end as f64 - start as f64) * progress;
    value.round().clamp(0.0, 255.0) as u8
}
