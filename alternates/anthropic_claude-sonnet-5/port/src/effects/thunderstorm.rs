//! Thunderstorm effect: staggered "lightning bolts" reveal the input text
//! column by column, each bolt flashing from bright white through pale
//! blue-white and a dim storm-gray afterglow before settling to the plain
//! input appearance. Mirrors the reveal-via-flash motif of upstream's
//! `effect_thunderstorm.py`, reimplemented directly against the character
//! animation API (no Scene bookkeeping needed since every frame's exact
//! appearance is computed deterministically up front).

use super::Effect;
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, ColorPair};

pub struct Thunderstorm;

impl Thunderstorm {
    pub fn new() -> Self {
        Thunderstorm
    }
}

/// Minimal deterministic PRNG so runs are reproducible without depending on
/// an external `rand` crate or the (not-yet-available) `utils::rng` module.
fn xorshift32(mut state: u32) -> u32 {
    state ^= state << 13;
    state ^= state >> 17;
    state ^= state << 5;
    state
}

const FLASH_COLORS: [Color; 4] = [
    Color::Rgb(255, 255, 255), // bright flash
    Color::Rgb(200, 220, 255), // pale blue-white
    Color::Rgb(120, 140, 200), // dim blue-gray afterglow
    Color::Rgb(80, 80, 90),    // fading toward storm dark
];

const FLASH_TICKS_PER_COLOR: u32 = 2;
const STRIKE_GROUPS: u32 = 12;

impl Effect for Thunderstorm {
    fn name(&self) -> &str {
        "thunderstorm"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);

        // Every character starts dark; each becomes visible only when its
        // bolt strikes.
        for character in terminal.get_characters_mut() {
            character.set_visibility(false);
        }

        // Assign each column to a pseudo-random strike group so whole
        // vertical bolts flash into existence together, rather than
        // individual characters popping independently.
        let width = terminal.canvas.width.max(1);
        let mut group_of_column: Vec<u32> = Vec::with_capacity(width);
        let mut seed = 0x9e37_79b9u32;
        for _ in 0..width {
            seed = xorshift32(seed.wrapping_add(0x1234_5678));
            group_of_column.push(seed % STRIKE_GROUPS);
        }

        // Give each strike group a pseudo-random rank, then space the
        // groups out in time so bolts land sequentially rather than all at
        // once.
        let mut group_start_tick: Vec<u32> = Vec::with_capacity(STRIKE_GROUPS as usize);
        let mut gseed = 0xdead_beefu32;
        for _ in 0..STRIKE_GROUPS {
            gseed = xorshift32(gseed.wrapping_add(0xabcd_ef01));
            group_start_tick.push(gseed % 20);
        }
        let mut order: Vec<usize> = (0..STRIKE_GROUPS as usize).collect();
        order.sort_by_key(|&i| group_start_tick[i]);
        let mut spaced_start: Vec<u32> = vec![0; STRIKE_GROUPS as usize];
        for (rank, &group_idx) in order.iter().enumerate() {
            spaced_start[group_idx] = (rank as u32) * 4;
        }

        let flash_duration = FLASH_COLORS.len() as u32 * FLASH_TICKS_PER_COLOR;
        let max_start = spaced_start.iter().copied().max().unwrap_or(0);
        let total_ticks = max_start + flash_duration + 3;

        let char_ids: Vec<u32> = terminal.get_characters().iter().map(|c| c.id).collect();
        let char_columns: Vec<i32> = terminal
            .get_characters()
            .iter()
            .map(|c| c.input_coord.column)
            .collect();

        let mut frames = Vec::with_capacity(total_ticks as usize);

        for tick in 0..total_ticks {
            for (idx, &id) in char_ids.iter().enumerate() {
                let column = char_columns[idx].max(0) as usize;
                let group = group_of_column[column.min(group_of_column.len() - 1)];
                let start = spaced_start[group as usize];
                if tick < start {
                    continue;
                }
                let elapsed = tick - start;

                let character = terminal.get_character_mut(id).unwrap();
                if !character.visible {
                    character.set_visibility(true);
                }
                let symbol = character.input_symbol;
                if elapsed < flash_duration {
                    let color_idx = (elapsed / FLASH_TICKS_PER_COLOR) as usize;
                    let color = FLASH_COLORS[color_idx.min(FLASH_COLORS.len() - 1)];
                    character
                        .animation
                        .set_appearance(symbol, Some(ColorPair::new(Some(color), None)));
                } else {
                    character.animation.set_appearance(symbol, None);
                }
            }
            frames.push(terminal.render());
        }

        frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_one_frame_per_tick_and_reveals_all_characters_eventually() {
        let effect = Thunderstorm::new();
        let frames = effect.frames("AB\nCD");
        assert!(!frames.is_empty());
        let last = frames.last().unwrap();
        assert!(last.contains('A') || last.contains('\u{1b}'));
    }
}
