use std::collections::{BTreeMap, HashMap, VecDeque};

use super::Effect;
use crate::engine::character::{CharacterId, EffectCharacter};
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Direction the wipe travels while revealing characters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WipeDirection {
    ColumnLeftToRight,
    ColumnRightToLeft,
    RowTopToBottom,
    RowBottomToTop,
    DiagonalTopLeftToBottomRight,
    DiagonalBottomLeftToTopRight,
    DiagonalTopRightToBottomLeft,
    DiagonalBottomRightToTopLeft,
    CenterToOutside,
    OutsideToCenter,
}

/// Wipes the text across the terminal to reveal characters.
pub struct Wipe {
    wipe_direction: WipeDirection,
    final_gradient_stops: Vec<Color>,
    final_gradient_steps: usize,
    final_gradient_frames: usize,
    wipe_delay: usize,
}

impl Wipe {
    pub fn new() -> Self {
        Self {
            wipe_direction: WipeDirection::ColumnLeftToRight,
            final_gradient_stops: vec![Color::rgb(0x83, 0x38, 0xec), Color::rgb(0x3a, 0x86, 0xff)],
            final_gradient_steps: 12,
            final_gradient_frames: 5,
            wipe_delay: 0,
        }
    }
}

impl Default for Wipe {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Wipe {
    fn name(&self) -> &str {
        "wipe"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        if term.character_count() == 0 {
            return Vec::new();
        }

        let hold = self.final_gradient_frames.max(1);
        let (mut pending, palettes) = {
            let chars = term.get_characters();
            let text_left = chars.iter().map(|ch| ch.input_coord.column).min().unwrap_or(1);
            let text_right = chars.iter().map(|ch| ch.input_coord.column).max().unwrap_or(1);
            let text_bottom = chars.iter().map(|ch| ch.input_coord.row).min().unwrap_or(1);
            let text_top = chars.iter().map(|ch| ch.input_coord.row).max().unwrap_or(1);

            let final_gradient = Gradient::new(&self.final_gradient_stops, self.final_gradient_steps);
            let row_delta = text_top - text_bottom;
            let fallback = self
                .final_gradient_stops
                .last()
                .copied()
                .unwrap_or(Color::rgb(0xff, 0xff, 0xff));

            let mut palettes: HashMap<CharacterId, Vec<Color>> = HashMap::new();
            for ch in chars {
                let progress = if row_delta == 0 {
                    0.0
                } else {
                    f64::from(text_top - ch.input_coord.row) / f64::from(row_delta)
                };
                let final_color = final_gradient.mapped_color(progress).unwrap_or(fallback);
                let fade = Gradient::new(&[Color::rgb(0, 0, 0), final_color], self.final_gradient_steps);
                let spectrum = fade.spectrum();
                palettes.insert(
                    ch.id,
                    if spectrum.is_empty() {
                        vec![final_color]
                    } else {
                        spectrum.to_vec()
                    },
                );
            }

            let groups = group_characters(
                chars,
                self.wipe_direction,
                text_left,
                text_right,
                text_bottom,
                text_top,
            );
            (VecDeque::from(groups), palettes)
        };

        let mut active: HashMap<CharacterId, usize> = HashMap::new();
        let mut delay_left = 0usize;
        let mut frames = Vec::new();

        while !pending.is_empty() || !active.is_empty() {
            if delay_left == 0 {
                if let Some(group) = pending.pop_front() {
                    for id in group {
                        term.set_character_visibility(id, true);
                        active.insert(id, 0);
                    }
                }
                delay_left = self.wipe_delay;
            } else {
                delay_left -= 1;
            }

            term.tick();

            let updates: Vec<(CharacterId, Color)> = active
                .iter()
                .filter_map(|(&id, &tick)| {
                    let palette = palettes.get(&id)?;
                    if palette.is_empty() {
                        return None;
                    }
                    let idx = (tick / hold).min(palette.len() - 1);
                    Some((id, palette[idx]))
                })
                .collect();
            for (id, color) in updates {
                if let Some(ch) = term.get_character_mut(id) {
                    let symbol = ch.input_symbol.clone();
                    ch.animation.set_appearance(&symbol, Some(ColorPair::fg(color)));
                }
            }

            let mut finished = Vec::new();
            for (&id, tick) in &mut active {
                *tick += 1;
                let limit = palettes
                    .get(&id)
                    .map(|palette| palette.len().saturating_mul(hold))
                    .unwrap_or(0);
                if *tick >= limit {
                    finished.push(id);
                }
            }
            for id in finished {
                active.remove(&id);
            }

            frames.push(term.render_frame());
        }

        frames
    }
}

fn group_characters(
    chars: &[EffectCharacter],
    direction: WipeDirection,
    text_left: i32,
    text_right: i32,
    text_bottom: i32,
    text_top: i32,
) -> Vec<Vec<CharacterId>> {
    let center_column = text_left + (text_right - text_left) / 2;
    let center_row = text_bottom + (text_top - text_bottom) / 2;
    let key = |ch: &EffectCharacter| -> i32 {
        let coord = ch.input_coord;
        match direction {
            WipeDirection::ColumnLeftToRight | WipeDirection::ColumnRightToLeft => coord.column,
            WipeDirection::RowTopToBottom | WipeDirection::RowBottomToTop => coord.row,
            WipeDirection::DiagonalTopLeftToBottomRight
            | WipeDirection::DiagonalBottomRightToTopLeft => coord.column - coord.row,
            WipeDirection::DiagonalBottomLeftToTopRight
            | WipeDirection::DiagonalTopRightToBottomLeft => coord.column + coord.row,
            WipeDirection::CenterToOutside | WipeDirection::OutsideToCenter => (coord.column
                - center_column)
                .abs()
                .max((coord.row - center_row).abs()),
        }
    };
    let reverse = matches!(
        direction,
        WipeDirection::ColumnRightToLeft
            | WipeDirection::RowTopToBottom
            | WipeDirection::DiagonalBottomRightToTopLeft
            | WipeDirection::DiagonalTopRightToBottomLeft
            | WipeDirection::OutsideToCenter
    );

    let mut buckets: BTreeMap<i32, Vec<CharacterId>> = BTreeMap::new();
    for ch in chars {
        buckets.entry(key(ch)).or_default().push(ch.id);
    }
    let mut groups: Vec<Vec<CharacterId>> = buckets.into_values().collect();
    if reverse {
        groups.reverse();
    }
    groups
}
