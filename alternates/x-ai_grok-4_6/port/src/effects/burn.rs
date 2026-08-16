//! Burn effect — characters ignite from the bottom of each column,
//! heat through the fire palette, wipe back down, then fade into
//! the final vertical gradient. Mirrors terminaltexteffects `effect_burn`.

use std::collections::BTreeMap;

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair, Gradient};

const STARTING_HEX: &str = "837373";
const BURN_HEX: [&str; 5] = ["ffffff", "fff75d", "fe650d", "8a003c", "510100"];
const FINAL_HEX: [&str; 3] = ["8A003C", "00D1FF", "FFFFFF"];
const FIRE_STEPS: usize = 10;
const FADE_STEPS: usize = 8;
const HEAT_HOLD: u32 = 20;
const WIPE_HOLD: u32 = 5;
const FADE_HOLD: u32 = 12;

pub struct Burn;

impl Burn {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Burn {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Burn {
    fn name(&self) -> &str {
        "burn"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_input(input, TerminalConfig::default());
        let count = terminal.character_count();
        if count == 0 {
            return vec![terminal.render_frame()];
        }

        let starting = hex(STARTING_HEX);
        let fire_stops: Vec<Color> = BURN_HEX.iter().copied().map(hex).collect();
        let fire = Gradient::new(&fire_stops, FIRE_STEPS);
        let fire_spectrum: Vec<Color> = fire.spectrum().to_vec();

        let final_stops: Vec<Color> = FINAL_HEX.iter().copied().map(hex).collect();
        let final_gradient = Gradient::new(&final_stops, 12);

        let (bottom, top) = text_row_bounds(&terminal);

        let final_colors: Vec<Color> = terminal
            .get_characters()
            .iter()
            .map(|ch| {
                let progress = vertical_progress(ch.input_coord.row, bottom, top);
                final_gradient.mapped_color(progress).unwrap_or(starting)
            })
            .collect();

        let timelines: Vec<Vec<(Color, u32)>> = final_colors
            .iter()
            .map(|color| burn_timeline(&fire_spectrum, *color))
            .collect();

        terminal.show_all();
        for index in 0..count {
            paint(&mut terminal, index, starting);
        }

        let pending = column_pending(&terminal);
        let mut next_pending = 0usize;
        let mut active: Vec<ActiveBurn> = Vec::new();
        let mut frames = Vec::new();

        loop {
            if next_pending < pending.len() {
                let index = pending[next_pending];
                next_pending += 1;
                if let Some(&(color, hold)) = timelines[index].first() {
                    paint(&mut terminal, index, color);
                    active.push(ActiveBurn {
                        index,
                        stage: 0,
                        remaining: hold,
                    });
                }
            }

            frames.push(terminal.render_frame());

            let mut still = Vec::with_capacity(active.len());
            for mut burner in active.drain(..) {
                if burner.remaining > 0 {
                    burner.remaining -= 1;
                }
                if burner.remaining == 0 {
                    burner.stage += 1;
                    if let Some(&(color, hold)) = timelines[burner.index].get(burner.stage) {
                        paint(&mut terminal, burner.index, color);
                        burner.remaining = hold;
                        still.push(burner);
                    }
                } else {
                    still.push(burner);
                }
            }
            active = still;

            if next_pending >= pending.len() && active.is_empty() {
                break;
            }
        }

        frames
    }
}

struct ActiveBurn {
    index: usize,
    stage: usize,
    remaining: u32,
}

fn hex(value: &str) -> Color {
    Color::from_hex(value).unwrap_or(Color::rgb(255, 255, 255))
}

fn paint(terminal: &mut Terminal, index: usize, color: Color) {
    let symbol = terminal
        .get_characters()
        .get(index)
        .map(|ch| ch.input_symbol.clone())
        .unwrap_or_else(|| " ".to_string());
    if let Some(ch) = terminal.get_characters_mut().get_mut(index) {
        ch.animation
            .set_appearance(&symbol, Some(ColorPair::fg(color)));
    }
}

fn text_row_bounds(terminal: &Terminal) -> (i32, i32) {
    let mut bottom = i32::MAX;
    let mut top = i32::MIN;
    for ch in terminal.get_characters() {
        bottom = bottom.min(ch.input_coord.row);
        top = top.max(ch.input_coord.row);
    }
    if bottom > top {
        (0, 0)
    } else {
        (bottom, top)
    }
}

fn vertical_progress(row: i32, bottom: i32, top: i32) -> f64 {
    let span = top - bottom;
    if span == 0 {
        0.0
    } else {
        f64::from(row - bottom) / f64::from(span)
    }
}

fn column_pending(terminal: &Terminal) -> Vec<usize> {
    let mut columns: BTreeMap<i32, Vec<(i32, usize)>> = BTreeMap::new();
    for (index, ch) in terminal.get_characters().iter().enumerate() {
        columns
            .entry(ch.input_coord.column)
            .or_default()
            .push((ch.input_coord.row, index));
    }
    let mut pending = Vec::new();
    for mut group in columns.into_values() {
        // COLUMN_LEFT_TO_RIGHT is top-first, then the effect reverses each
        // column so ignition starts at the bottom (lowest row) and climbs.
        group.sort_by_key(|(row, _)| *row);
        pending.extend(group.into_iter().map(|(_, index)| index));
    }
    pending
}

fn burn_timeline(fire: &[Color], final_color: Color) -> Vec<(Color, u32)> {
    let mut timeline = Vec::new();
    if fire.is_empty() {
        timeline.push((final_color, FADE_HOLD));
        return timeline;
    }
    for &color in fire.iter().rev() {
        timeline.push((color, HEAT_HOLD));
    }
    for &color in fire {
        timeline.push((color, WIPE_HOLD));
    }
    let last_fire = *fire.last().unwrap_or(&final_color);
    let fade = Gradient::new(&[last_fire, final_color], FADE_STEPS);
    if fade.is_empty() {
        timeline.push((final_color, FADE_HOLD));
    } else {
        for &color in fade.spectrum() {
            timeline.push((color, FADE_HOLD));
        }
    }
    timeline
}
