
use std::collections::BTreeMap;

use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::Terminal;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Style};

/// Reveals the input diagonally from the bottom-left toward the top-right.
#[derive(Debug, Clone, Copy, Default)]
pub struct Wipe;

impl Wipe {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Wipe {
    fn name(&self) -> &str {
        "wipe"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);

        if terminal.characters().is_empty() {
            return Vec::new();
        }

        let width = terminal.canvas().width();
        let height = terminal.canvas().height();

        // The upstream default final gradient runs vertically through these
        // three colors.
        const GRADIENT_STOPS: [Color; 3] = [
            Color::rgb(0x83, 0x3a, 0xb4),
            Color::rgb(0xfd, 0x1d, 0x1d),
            Color::rgb(0xfc, 0xb0, 0x45),
        ];

        let mut groups: BTreeMap<i32, Vec<CharacterId>> = BTreeMap::new();

        for character in terminal.characters_mut() {
            let coord = character.position;
            let color = vertical_gradient_color(coord, height, &GRADIENT_STOPS);

            character.set_appearance(
                character.input_symbol,
                Style::default().with_foreground(color),
            );
            character.visible = false;

            // Rust canvas rows increase downward. Converting y to its distance
            // from the bottom makes ascending keys travel bottom-left to
            // top-right.
            let row_from_bottom = height.saturating_sub(1) as i32 - coord.y;
            groups
                .entry(coord.x + row_from_bottom)
                .or_default()
                .push(character.id);
        }

        for group in groups.values_mut() {
            group.sort_unstable();
        }

        let mut frames = Vec::with_capacity(groups.len());

        for group in groups.into_values() {
            for id in group {
                if let Some(character) = terminal.character_mut(id) {
                    character.visible = true;
                }
            }

            frames.push(terminal.render_frame());
        }

        // Width is intentionally read above along with height so dimensions
        // remain explicit even for one-cell canvases.
        let _ = width;

        frames
    }
}

fn vertical_gradient_color(coord: Coord, height: usize, stops: &[Color]) -> Color {
    if stops.is_empty() {
        return Color::rgb(255, 255, 255);
    }

    if stops.len() == 1 {
        return stops[0];
    }

    // The terminal's first input row is the visual top. The upstream canvas
    // uses upward-growing rows, so reverse the ratio to preserve its vertical
    // gradient orientation.
    let ratio = if height <= 1 {
        1.0
    } else {
        1.0 - coord.y.clamp(0, height as i32 - 1) as f64 / (height - 1) as f64
    };

    let scaled = ratio.clamp(0.0, 1.0) * (stops.len() - 1) as f64;
    let segment = (scaled.floor() as usize).min(stops.len() - 2);
    let progress = scaled - segment as f64;

    interpolate_color(stops[segment], stops[segment + 1], progress)
}

fn interpolate_color(start: Color, end: Color, progress: f64) -> Color {
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
        (Color::Ansi(start), Color::Ansi(end)) => {
            Color::ansi(interpolate_channel(start, end, progress))
        }
        (_, end) if progress >= 0.5 => end,
        (start, _) => start,
    }
}

fn interpolate_channel(start: u8, end: u8, progress: f64) -> u8 {
    let value = start as f64 + (end as f64 - start as f64) * progress.clamp(0.0, 1.0);
    value.round().clamp(0.0, 255.0) as u8
}
