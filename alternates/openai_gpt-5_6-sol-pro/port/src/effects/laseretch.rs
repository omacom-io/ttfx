
use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::Terminal;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Style};

const COOLING_FRAMES: u32 = 12;

pub struct Laseretch;

impl Laseretch {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Laseretch {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Laseretch {
    fn name(&self) -> &str {
        "laseretch"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);
        let width = terminal.canvas().width();
        let height = terminal.canvas().height();

        let originals = terminal
            .characters()
            .iter()
            .filter(|character| !character.input_symbol.is_whitespace())
            .map(|character| OriginalCharacter {
                id: character.id,
                symbol: character.input_symbol,
                coord: character.position,
                final_color: final_color(character.position, height),
            })
            .collect::<Vec<_>>();

        for character in terminal.characters_mut() {
            if !character.input_symbol.is_whitespace() {
                character.visible = false;
            }
        }

        if originals.is_empty() {
            return vec![terminal.render_frame()];
        }

        let mut beam_ids = Vec::with_capacity(width);
        for x in 0..width {
            let id = terminal.add_character('━', Coord::new(x as i32, 0));
            if let Some(character) = terminal.character_mut(id) {
                character.visible = false;
                character.set_style(laser_style(x, width));
            }
            beam_ids.push(id);
        }

        let head_id = terminal.add_character('█', Coord::ZERO);
        if let Some(head) = terminal.character_mut(head_id) {
            head.visible = false;
            head.set_style(Style {
                bold: true,
                ..Style::default().with_foreground(Color::rgb(255, 255, 180))
            });
        }

        let spark_symbols = ['*', '+', '·', '•'];
        let mut spark_ids = Vec::with_capacity(spark_symbols.len());
        for symbol in spark_symbols {
            let id = terminal.add_character(symbol, Coord::ZERO);
            if let Some(spark) = terminal.character_mut(id) {
                spark.visible = false;
            }
            spark_ids.push(id);
        }

        let mut frames = Vec::new();
        let mut cooling = Vec::<CoolingCharacter>::new();

        for row in 0..height {
            let row_y = row as i32;
            let leftmost = originals
                .iter()
                .filter(|character| character.coord.y == row_y)
                .map(|character| character.coord.x)
                .min();

            let Some(leftmost) = leftmost else {
                continue;
            };

            let rightmost = originals
                .iter()
                .filter(|character| character.coord.y == row_y)
                .map(|character| character.coord.x)
                .max()
                .unwrap_or(leftmost);

            let entry_x = (width.saturating_sub(1)) as i32;

            for x in (leftmost..=entry_x).rev() {
                advance_cooling(&mut terminal, &mut cooling);
                hide_sparks(&mut terminal, &spark_ids);
                position_laser(&mut terminal, &beam_ids, head_id, Coord::new(x, row_y));

                if x <= rightmost {
                    if let Some(target) = originals
                        .iter()
                        .find(|character| character.coord == Coord::new(x, row_y))
                    {
                        if let Some(character) = terminal.character_mut(target.id) {
                            character.visible = true;
                            character.set_appearance(
                                target.symbol,
                                Style {
                                    bold: true,
                                    ..Style::default()
                                        .with_foreground(Color::rgb(255, 255, 255))
                                },
                            );
                        }

                        cooling.push(CoolingCharacter {
                            id: target.id,
                            symbol: target.symbol,
                            final_color: target.final_color,
                            age: 0,
                        });

                        position_sparks(
                            &mut terminal,
                            &spark_ids,
                            target.coord,
                            0,
                            width,
                            height,
                        );
                        frames.push(terminal.render_frame());

                        advance_cooling(&mut terminal, &mut cooling);
                        position_sparks(
                            &mut terminal,
                            &spark_ids,
                            target.coord,
                            1,
                            width,
                            height,
                        );
                        frames.push(terminal.render_frame());
                        continue;
                    }
                }

                frames.push(terminal.render_frame());
            }

            hide_laser(&mut terminal, &beam_ids, head_id, &spark_ids);
            advance_cooling(&mut terminal, &mut cooling);
            frames.push(terminal.render_frame());
        }

        hide_laser(&mut terminal, &beam_ids, head_id, &spark_ids);

        while !cooling.is_empty() {
            advance_cooling(&mut terminal, &mut cooling);
            frames.push(terminal.render_frame());
        }

        for original in &originals {
            if let Some(character) = terminal.character_mut(original.id) {
                character.visible = true;
                character.set_appearance(
                    original.symbol,
                    Style::default().with_foreground(original.final_color),
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

#[derive(Clone, Copy)]
struct OriginalCharacter {
    id: CharacterId,
    symbol: char,
    coord: Coord,
    final_color: Color,
}

struct CoolingCharacter {
    id: CharacterId,
    symbol: char,
    final_color: Color,
    age: u32,
}

fn position_laser(
    terminal: &mut Terminal,
    beam_ids: &[CharacterId],
    head_id: CharacterId,
    position: Coord,
) {
    for (x, id) in beam_ids.iter().copied().enumerate() {
        if let Some(beam) = terminal.character_mut(id) {
            beam.position = Coord::new(x as i32, position.y);
            beam.visible = x as i32 > position.x;
        }
    }

    if let Some(head) = terminal.character_mut(head_id) {
        head.position = position;
        head.visible = true;
    }
}

fn hide_laser(
    terminal: &mut Terminal,
    beam_ids: &[CharacterId],
    head_id: CharacterId,
    spark_ids: &[CharacterId],
) {
    for id in beam_ids {
        if let Some(character) = terminal.character_mut(*id) {
            character.visible = false;
        }
    }

    if let Some(head) = terminal.character_mut(head_id) {
        head.visible = false;
    }

    hide_sparks(terminal, spark_ids);
}

fn hide_sparks(terminal: &mut Terminal, spark_ids: &[CharacterId]) {
    for id in spark_ids {
        if let Some(spark) = terminal.character_mut(*id) {
            spark.visible = false;
        }
    }
}

fn position_sparks(
    terminal: &mut Terminal,
    spark_ids: &[CharacterId],
    origin: Coord,
    phase: usize,
    width: usize,
    height: usize,
) {
    let offsets = if phase == 0 {
        [
            Coord::new(0, -1),
            Coord::new(1, 0),
            Coord::new(0, 1),
            Coord::new(-1, 0),
        ]
    } else {
        [
            Coord::new(-1, -1),
            Coord::new(1, -1),
            Coord::new(1, 1),
            Coord::new(-1, 1),
        ]
    };

    let colors = [
        Color::rgb(255, 255, 255),
        Color::rgb(255, 255, 0),
        Color::rgb(255, 128, 0),
        Color::rgb(255, 32, 0),
    ];

    for (index, id) in spark_ids.iter().copied().enumerate() {
        let coord = origin + offsets[index];
        let is_visible = coord.x >= 0
            && coord.y >= 0
            && (coord.x as usize) < width
            && (coord.y as usize) < height;

        if let Some(spark) = terminal.character_mut(id) {
            spark.position = coord;
            spark.visible = is_visible;
            spark.set_style(
                Style {
                    bold: index < 2,
                    ..Style::default()
                }
                .with_foreground(colors[index]),
            );
        }
    }
}

fn advance_cooling(terminal: &mut Terminal, cooling: &mut Vec<CoolingCharacter>) {
    let mut index = 0;

    while index < cooling.len() {
        cooling[index].age = cooling[index].age.saturating_add(1);

        let age = cooling[index].age;
        let id = cooling[index].id;
        let symbol = cooling[index].symbol;
        let final_color = cooling[index].final_color;
        let color = cooling_color(age, final_color);

        if let Some(character) = terminal.character_mut(id) {
            character.visible = true;
            character.set_appearance(
                symbol,
                Style {
                    bold: age < COOLING_FRAMES / 2,
                    ..Style::default().with_foreground(color)
                },
            );
        }

        if age >= COOLING_FRAMES {
            cooling.remove(index);
        } else {
            index += 1;
        }
    }
}

fn cooling_color(age: u32, final_color: Color) -> Color {
    let progress = (age as f64 / COOLING_FRAMES as f64).clamp(0.0, 1.0);

    if progress < 0.2 {
        mix_color(
            Color::rgb(255, 255, 255),
            Color::rgb(255, 255, 0),
            progress / 0.2,
        )
    } else if progress < 0.5 {
        mix_color(
            Color::rgb(255, 255, 0),
            Color::rgb(255, 128, 0),
            (progress - 0.2) / 0.3,
        )
    } else if progress < 0.75 {
        mix_color(
            Color::rgb(255, 128, 0),
            Color::rgb(255, 0, 0),
            (progress - 0.5) / 0.25,
        )
    } else {
        mix_color(
            Color::rgb(255, 0, 0),
            final_color,
            (progress - 0.75) / 0.25,
        )
    }
}

fn laser_style(x: usize, width: usize) -> Style {
    let progress = if width <= 1 {
        1.0
    } else {
        x as f64 / (width - 1) as f64
    };

    Style {
        bold: true,
        ..Style::default().with_foreground(mix_color(
            Color::rgb(255, 32, 0),
            Color::rgb(255, 255, 0),
            progress,
        ))
    }
}

fn final_color(coord: Coord, height: usize) -> Color {
    let progress = if height <= 1 {
        1.0
    } else {
        (coord.y as f64 / (height - 1) as f64).clamp(0.0, 1.0)
    };

    if progress < 0.5 {
        mix_color(
            Color::rgb(138, 0, 138),
            Color::rgb(0, 209, 255),
            progress * 2.0,
        )
    } else {
        mix_color(
            Color::rgb(0, 209, 255),
            Color::rgb(255, 255, 255),
            (progress - 0.5) * 2.0,
        )
    }
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
        ) => {
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
        _ if progress < 0.5 => start,
        _ => end,
    }
}
