
use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, Style};

pub struct Sweep;

impl Sweep {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Sweep {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Sweep {
    fn name(&self) -> &str {
        "sweep"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);

        if terminal.characters().is_empty() {
            return Vec::new();
        }

        for character in terminal.characters_mut() {
            character.visible = false;
        }

        let width = terminal.canvas().width();
        let height = terminal.canvas().height();
        let mut columns = vec![Vec::<CharacterId>::new(); width];

        for character in terminal.characters() {
            if character.position.x >= 0 {
                let column = character.position.x as usize;
                if column < width {
                    columns[column].push(character.id);
                }
            }
        }

        let mut frames = Vec::with_capacity(width.saturating_mul(2));

        // The first sweep reveals the text in a bright, uniform color.
        for column in 0..width {
            for id in columns[column].iter().copied() {
                if let Some(character) = terminal.character_mut(id) {
                    character.visible = true;
                    character.set_appearance(
                        character.input_symbol,
                        Style::default().with_foreground(Color::rgb(255, 255, 255)),
                    );
                }
            }

            frames.push(terminal.render_frame());
        }

        // The return sweep applies the final vertical gradient.
        for column in (0..width).rev() {
            for id in columns[column].iter().copied() {
                let Some(position) = terminal.character(id).map(|character| character.position)
                else {
                    continue;
                };

                let color = final_gradient_color(position.y, height);

                if let Some(character) = terminal.character_mut(id) {
                    character.set_appearance(
                        character.input_symbol,
                        Style::default().with_foreground(color),
                    );
                }
            }

            frames.push(terminal.render_frame());
        }

        frames
    }
}

fn final_gradient_color(row: i32, height: usize) -> Color {
    const PURPLE: (u8, u8, u8) = (138, 0, 138);
    const CYAN: (u8, u8, u8) = (0, 209, 255);
    const WHITE: (u8, u8, u8) = (255, 255, 255);

    if height <= 1 {
        return Color::rgb(CYAN.0, CYAN.1, CYAN.2);
    }

    // Canvas rows increase downward. Reversing the ratio places white at the
    // top and purple at the bottom, matching the vertical final gradient.
    let ratio = 1.0 - (row.max(0) as f64 / (height - 1) as f64).clamp(0.0, 1.0);

    let (start, end, progress) = if ratio < 0.5 {
        (PURPLE, CYAN, ratio * 2.0)
    } else {
        (CYAN, WHITE, (ratio - 0.5) * 2.0)
    };

    Color::rgb(
        interpolate_channel(start.0, end.0, progress),
        interpolate_channel(start.1, end.1, progress),
        interpolate_channel(start.2, end.2, progress),
    )
}

fn interpolate_channel(start: u8, end: u8, progress: f64) -> u8 {
    let value = f64::from(start) + (f64::from(end) - f64::from(start)) * progress;
    value.round().clamp(0.0, 255.0) as u8
}
