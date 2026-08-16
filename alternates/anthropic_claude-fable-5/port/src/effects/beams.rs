//! Beams effect (port of terminaltexteffects/effects/effect_beams.py).
//!
//! Light beams travel across rows and columns of the input text, illuminating
//! each character as they pass and leaving it dimly lit. Once every beam has
//! finished, a final wipe sweeps from the top row to the bottom, brightening
//! every character to its final gradient color.

use std::collections::{BTreeMap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

use super::Effect;
use crate::engine::animation::Scene;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair, Gradient};

// --- upstream defaults (BeamsConfig) -------------------------------------

const BEAM_ROW_SYMBOLS: &[char] = &['▂', '▁', '_'];
const BEAM_COLUMN_SYMBOLS: &[char] = &['▌', '▍', '▎', '▏'];
const BEAM_DELAY: u32 = 10;
const BEAM_ROW_SPEED_RANGE: (u64, u64) = (10, 40);
const BEAM_COLUMN_SPEED_RANGE: (u64, u64) = (6, 10);
const BEAM_GRADIENT_FRAMES: u32 = 2;
const FINAL_GRADIENT_FRAMES: u32 = 5;
const FINAL_WIPE_SPEED: usize = 1;
const MAX_FRAMES: usize = 20_000;

fn white() -> Color {
    Color::new(0xFF, 0xFF, 0xFF)
}

fn cyan() -> Color {
    Color::new(0x00, 0xD1, 0xFF)
}

fn purple() -> Color {
    Color::new(0x8A, 0x00, 0x8A)
}

// --- tiny deterministic-fallback PRNG (no rand crate in this port) --------

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Rng(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Inclusive-range integer, mirroring Python's random.randint.
    fn randint(&mut self, lo: u64, hi: u64) -> u64 {
        if hi <= lo {
            return lo;
        }
        lo + self.next_u64() % (hi - lo + 1)
    }

    /// Mirrors Python's random.choice([True, False]).
    fn coin(&mut self) -> bool {
        self.next_u64() & 1 == 0
    }

    /// Fisher-Yates shuffle, mirroring Python's random.shuffle.
    fn shuffle<T>(&mut self, items: &mut [T]) {
        if items.len() < 2 {
            return;
        }
        for i in (1..items.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            items.swap(i, j);
        }
    }
}

// --- helpers ---------------------------------------------------------------

/// Port of Scene.apply_gradient_to_symbols: one frame per gradient color,
/// progressing through the symbols proportionally to gradient progress.
fn apply_gradient_to_symbols(scene: &mut Scene, symbols: &[char], duration: u32, gradient: &Gradient) {
    if symbols.is_empty() || gradient.spectrum.is_empty() {
        return;
    }
    let n = gradient.spectrum.len();
    for (i, color) in gradient.spectrum.iter().enumerate() {
        let symbol_index = if n <= 1 {
            symbols.len() - 1
        } else {
            ((i as f64 / (n - 1) as f64) * (symbols.len() - 1) as f64).round() as usize
        };
        let symbol = symbols[symbol_index.min(symbols.len() - 1)];
        scene.add_frame(symbol, duration, ColorPair::fg(*color), false);
    }
}

/// Approximation of Animation.adjust_color_brightness(color, factor).
fn adjust_brightness(color: Color, factor: f64) -> Color {
    let scale = |c: u8| (c as f64 * factor).round().clamp(0.0, 255.0) as u8;
    Color::new(scale(color.r), scale(color.g), scale(color.b))
}

/// Build the beam gradient with the upstream per-segment steps (2, 8).
fn beam_gradient() -> Gradient {
    let first = Gradient::new(&[white(), cyan()], 2);
    let second = Gradient::new(&[cyan(), purple()], 8);
    let mut spectrum = first.spectrum;
    spectrum.extend(second.spectrum.into_iter().skip(1));
    Gradient { spectrum }
}

/// One beam sweeping along a row or column; mirrors BeamsIterator.Group.
struct Group {
    /// Character ids remaining to be lit, popped from the front.
    ids: Vec<usize>,
    /// Scene to activate on each character ("beam_row" or "beam_column").
    scene: &'static str,
    speed: f64,
    counter: f64,
}

fn activate_scene_on(terminal: &mut Terminal, id: usize, scene: &str) {
    terminal.set_character_visibility(id, true);
    if let Some(character) = terminal
        .get_characters_mut()
        .iter_mut()
        .find(|c| c.character_id == id)
    {
        character.animation.activate_scene(scene);
    }
}

// --- effect ------------------------------------------------------------------

pub struct Beams;

impl Beams {
    pub fn new() -> Self {
        Beams
    }
}

impl Default for Beams {
    fn default() -> Self {
        Beams::new()
    }
}

impl Effect for Beams {
    fn name(&self) -> &str {
        "beams"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let mut rng = Rng::new();
        let height = terminal.canvas.height as i32;

        // -- build (mirrors BeamsIterator.build) --

        // Final gradient, mapped vertically: purple at the bottom, white on top.
        let final_gradient = Gradient::new(&[purple(), cyan(), white()], 12);
        let mut final_colors: HashMap<usize, Color> = HashMap::new();
        for character in terminal.get_characters() {
            let fraction = if height > 1 {
                (character.input_coord.row - 1) as f64 / (height - 1) as f64
            } else {
                1.0
            };
            let color = final_gradient
                .get_color_at_fraction(fraction)
                .unwrap_or_else(white);
            final_colors.insert(character.character_id, color);
        }

        let beam_gradient = beam_gradient();

        // Per-character scenes: beam_row, beam_column, brighten.
        for character in terminal.get_characters_mut() {
            let final_color = final_colors
                .get(&character.character_id)
                .copied()
                .unwrap_or_else(white);
            let faded_color = adjust_brightness(final_color, 0.3);
            let fade_gradient = Gradient::new(&[purple(), faded_color], 10);
            let brighten_gradient = Gradient::new(&[faded_color, final_color], 10);
            let input_symbol = character.input_symbol;

            {
                let scene = character.animation.new_scene("beam_row", false);
                apply_gradient_to_symbols(scene, BEAM_ROW_SYMBOLS, BEAM_GRADIENT_FRAMES, &beam_gradient);
                apply_gradient_to_symbols(scene, &[input_symbol], 5, &fade_gradient);
            }
            {
                let scene = character.animation.new_scene("beam_column", false);
                apply_gradient_to_symbols(scene, BEAM_COLUMN_SYMBOLS, BEAM_GRADIENT_FRAMES, &beam_gradient);
                apply_gradient_to_symbols(scene, &[input_symbol], 5, &fade_gradient);
            }
            {
                let scene = character.animation.new_scene("brighten", false);
                apply_gradient_to_symbols(scene, &[input_symbol], FINAL_GRADIENT_FRAMES, &brighten_gradient);
            }
        }

        // Group characters by row (top to bottom) and column (left to right).
        let mut rows: BTreeMap<i32, Vec<(i32, usize)>> = BTreeMap::new();
        let mut columns: BTreeMap<i32, Vec<(i32, usize)>> = BTreeMap::new();
        for character in terminal.get_characters() {
            rows.entry(character.input_coord.row)
                .or_default()
                .push((character.input_coord.column, character.character_id));
            columns
                .entry(character.input_coord.column)
                .or_default()
                .push((character.input_coord.row, character.character_id));
        }

        // Final wipe groups: rows from the top down.
        let wipe_groups_src: Vec<Vec<usize>> = rows
            .iter()
            .rev()
            .map(|(_, members)| {
                let mut members = members.clone();
                members.sort();
                members.into_iter().map(|(_, id)| id).collect()
            })
            .collect();
        let mut wipe_groups = wipe_groups_src;

        let mut groups: Vec<Group> = Vec::new();
        for (_, mut members) in rows.into_iter().rev() {
            members.sort();
            let mut ids: Vec<usize> = members.into_iter().map(|(_, id)| id).collect();
            if rng.coin() {
                ids.reverse();
            }
            let speed = rng.randint(BEAM_ROW_SPEED_RANGE.0, BEAM_ROW_SPEED_RANGE.1) as f64 * 0.1;
            groups.push(Group {
                ids,
                scene: "beam_row",
                speed,
                counter: 0.0,
            });
        }
        for (_, mut members) in columns {
            members.sort();
            let mut ids: Vec<usize> = members.into_iter().map(|(_, id)| id).collect();
            if rng.coin() {
                ids.reverse();
            }
            let speed = rng.randint(BEAM_COLUMN_SPEED_RANGE.0, BEAM_COLUMN_SPEED_RANGE.1) as f64 * 0.1;
            groups.push(Group {
                ids,
                scene: "beam_column",
                speed,
                counter: 0.0,
            });
        }
        rng.shuffle(&mut groups);

        // -- run (mirrors BeamsIterator.__next__) --

        let mut active_groups = groups;
        let mut current_groups: Vec<Group> = Vec::new();
        let mut delay: u32 = 0;
        let mut phase_beams = true;
        let mut frames_out: Vec<String> = Vec::new();

        loop {
            if phase_beams {
                if delay == 0 {
                    if !active_groups.is_empty() {
                        let count = rng.randint(1, 5) as usize;
                        for _ in 0..count {
                            if active_groups.is_empty() {
                                break;
                            }
                            current_groups.push(active_groups.remove(0));
                        }
                    }
                    delay = BEAM_DELAY;
                } else {
                    delay -= 1;
                }

                for group in current_groups.iter_mut() {
                    group.counter += group.speed;
                    let releases = group.counter as i64;
                    if releases > 1 {
                        for _ in 0..releases {
                            if group.ids.is_empty() {
                                break;
                            }
                            group.counter -= 1.0;
                            let id = group.ids.remove(0);
                            activate_scene_on(&mut terminal, id, group.scene);
                        }
                    }
                }
                current_groups.retain(|group| !group.ids.is_empty());
            } else if !wipe_groups.is_empty() {
                for _ in 0..FINAL_WIPE_SPEED {
                    if wipe_groups.is_empty() {
                        break;
                    }
                    let ids = wipe_groups.remove(0);
                    for id in ids {
                        activate_scene_on(&mut terminal, id, "brighten");
                    }
                }
            }

            let active = terminal.tick();
            frames_out.push(terminal.get_formatted_output_string());

            if phase_beams {
                if current_groups.is_empty() && active_groups.is_empty() && active == 0 {
                    phase_beams = false;
                }
            } else if wipe_groups.is_empty() && active == 0 {
                break;
            }

            if frames_out.len() >= MAX_FRAMES {
                break;
            }
        }

        frames_out
    }
}
