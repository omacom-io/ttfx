//! Highlight effect — sweep a beam across the text and leave matches lit.
//!
//! Port of `terminaltexteffects/effects/effect_highlight.py`.
//!
//! Default config (empty pattern) treats every character as a match:
//! all characters start visible in a muted tone, a gold beam travels
//! left-to-right one column per frame, and each character flashes the
//! highlight color before settling on the match color.

use std::collections::HashMap;

use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair};

/// Color of the beam that highlights the text (`--highlight-color ffcb6b`).
const HIGHLIGHT_COLOR: Color = Color {
    r: 0xff,
    g: 0xcb,
    b: 0x6b,
};

/// Color of matched characters after the beam has passed (`--match-color 89ddff`).
const MATCH_COLOR: Color = Color {
    r: 0x89,
    g: 0xdd,
    b: 0xff,
};

/// Muted tone used before the beam reaches a character.
const UNREAD_COLOR: Color = Color {
    r: 0x6c,
    g: 0x70,
    b: 0x86,
};

/// Frames each character stays on the highlight color (Python scene duration).
const HIGHLIGHT_FRAMES: usize = 5;

/// Search the input and highlight every character with a sweeping beam.
pub struct Highlight;

impl Highlight {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Highlight {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Highlight {
    fn name(&self) -> &str {
        "highlight"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        if term.character_count() == 0 {
            return vec![term.render_frame()];
        }

        term.show_all();
        paint_all(&mut term, UNREAD_COLOR);

        let mut frames = Vec::new();
        frames.push(term.render_frame());

        // Column groups, left-to-right; within a column, top-to-bottom
        // (Python sort key is `(column, -row)`).
        let mut placed: Vec<(i32, i32, CharacterId)> = term
            .get_characters()
            .iter()
            .map(|ch| (ch.input_coord.column, ch.input_coord.row, ch.id))
            .collect();
        placed.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));

        let mut groups: Vec<Vec<CharacterId>> = Vec::new();
        for (column, _row, id) in placed {
            match groups.last_mut() {
                Some(group) if last_column_of(&term, group) == Some(column) => {
                    group.push(id);
                }
                _ => groups.push(vec![id]),
            }
        }

        // Remaining highlight frames per character. `0` means settled on match.
        let mut remaining: HashMap<CharacterId, usize> = HashMap::new();
        let mut group_idx = 0usize;

        loop {
            let activating = group_idx < groups.len();
            let beam_live = remaining.values().any(|&left| left > 0);
            if !activating && !beam_live {
                break;
            }

            if activating {
                for &id in &groups[group_idx] {
                    remaining.insert(id, HIGHLIGHT_FRAMES);
                }
                group_idx += 1;
            }

            for ch in term.get_characters_mut() {
                let color = match remaining.get(&ch.id).copied() {
                    Some(left) if left > 0 => HIGHLIGHT_COLOR,
                    Some(_) => MATCH_COLOR,
                    None => continue,
                };
                let symbol = ch.input_symbol.clone();
                ch.animation
                    .set_appearance(&symbol, Some(ColorPair::fg(color)));
            }
            frames.push(term.render_frame());

            for left in remaining.values_mut() {
                if *left > 0 {
                    *left -= 1;
                }
            }
        }

        // Ensure the settled match color is on screen for the last column.
        paint_ids(&mut term, remaining.keys().copied(), MATCH_COLOR);
        frames.push(term.render_frame());

        frames
    }
}

fn last_column_of(term: &Terminal, group: &[CharacterId]) -> Option<i32> {
    group
        .last()
        .copied()
        .and_then(|id| term.get_character(id).map(|ch| ch.input_coord.column))
}

fn paint_all(term: &mut Terminal, color: Color) {
    for ch in term.get_characters_mut() {
        let symbol = ch.input_symbol.clone();
        ch.animation
            .set_appearance(&symbol, Some(ColorPair::fg(color)));
    }
}

fn paint_ids<I>(term: &mut Terminal, ids: I, color: Color)
where
    I: IntoIterator<Item = CharacterId>,
{
    let mark: HashMap<CharacterId, ()> = ids.into_iter().map(|id| (id, ())).collect();
    for ch in term.get_characters_mut() {
        if mark.contains_key(&ch.id) {
            let symbol = ch.input_symbol.clone();
            ch.animation
                .set_appearance(&symbol, Some(ColorPair::fg(color)));
        }
    }
}
