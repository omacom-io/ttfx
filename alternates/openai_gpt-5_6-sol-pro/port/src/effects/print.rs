
use std::collections::BTreeMap;

use super::Effect;
use crate::engine::{CharacterId, Terminal};
use crate::utils::easing::in_out_quad;
use crate::utils::{Color, Coord, Style};

const PRINT_HEAD_SYMBOL: char = '█';
const PRINT_HEAD_RETURN_SPEED: f64 = 1.25;
const PRINT_SPEED: usize = 1;

const GRADIENT_BOTTOM: Color = Color::rgb(0x02, 0xb8, 0xbd);
const GRADIENT_TOP: Color = Color::rgb(0xc1, 0xf0, 0xe3);

#[derive(Debug, Clone, Copy, Default)]
pub struct Print;

impl Print {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Print {
    fn name(&self) -> &str {
        "print"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);
        let height = terminal.canvas().height();

        let mut rows: BTreeMap<i32, Vec<CharacterId>> = BTreeMap::new();
        for character in terminal.characters() {
            rows.entry(character.position.y)
                .or_default()
                .push(character.id);
        }

        for row in rows.values_mut() {
            row.sort_by_key(|id| {
                terminal
                    .character(*id)
                    .map(|character| (character.position.x, character.id))
                    .unwrap_or((i32::MAX, *id))
            });
        }

        for character in terminal.characters_mut() {
            character.visible = false;
            let color = gradient_color(character.position.y, height);
            character.set_style(Style::default().with_foreground(color));
        }

        let rows: Vec<(i32, Vec<CharacterId>)> = rows.into_iter().collect();
        if rows.is_empty() {
            return vec![terminal.render_frame()];
        }

        let first_row = rows[0].0;
        let print_head_id = terminal.add_character(
            PRINT_HEAD_SYMBOL,
            Coord::new(0, first_row),
        );

        if let Some(print_head) = terminal.character_mut(print_head_id) {
            print_head.set_style(
                Style::default().with_foreground(Color::rgb(0xff, 0xff, 0xff)),
            );
        }

        let mut frames = Vec::new();

        for (row_index, (row_y, character_ids)) in rows.iter().enumerate() {
            for batch in character_ids.chunks(PRINT_SPEED) {
                let mut print_head_x = 0;

                for id in batch {
                    if let Some(character) = terminal.character_mut(*id) {
                        character.visible = true;
                        print_head_x = character.position.x.saturating_add(1);
                    }
                }

                if let Some(print_head) = terminal.character_mut(print_head_id) {
                    print_head.set_position(Coord::new(print_head_x, *row_y));
                }

                frames.push(terminal.render_frame());
            }

            let Some((next_row_y, _)) = rows.get(row_index + 1) else {
                continue;
            };

            let start = terminal
                .character(print_head_id)
                .map(|character| character.position)
                .unwrap_or(Coord::new(0, *row_y));
            let destination = Coord::new(0, *next_row_y);
            let distance = start.distance(destination);
            let return_steps = (distance / PRINT_HEAD_RETURN_SPEED)
                .ceil()
                .max(1.0) as usize;

            for step in 1..=return_steps {
                let raw_progress = step as f64 / return_steps as f64;
                let position = start.lerp(destination, in_out_quad(raw_progress));

                if let Some(print_head) = terminal.character_mut(print_head_id) {
                    print_head.set_position(position);
                }

                frames.push(terminal.render_frame());
            }
        }

        if let Some(print_head) = terminal.character_mut(print_head_id) {
            print_head.visible = false;
        }
        frames.push(terminal.render_frame());

        frames
    }
}

fn gradient_color(row: i32, height: usize) -> Color {
    if height <= 1 {
        return GRADIENT_BOTTOM;
    }

    let row = row.clamp(0, height.saturating_sub(1) as i32) as f64;
    let progress = 1.0 - row / (height - 1) as f64;

    interpolate_color(GRADIENT_BOTTOM, GRADIENT_TOP, progress)
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
        return start;
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
