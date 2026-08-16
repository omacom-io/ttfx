//! Beams — port of `terminaltexteffects/effects/effect_beams.py`.
//!
//! Row and column groups sweep across the canvas, illuminating each character
//! with a short beam-glyph gradient, then fading to a dim final color. A
//! diagonal wipe then brightens every character into the final gradient.

use std::collections::{BTreeMap, HashMap, VecDeque};

use super::Effect;
use crate::engine::{CharacterId, Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair, Gradient};

const BEAM_ROW_SYMBOLS: &[&str] = &["▂", "▁", "_"];
const BEAM_COLUMN_SYMBOLS: &[&str] = &["▌", "▍", "▎", "▏"];
const BEAM_DELAY: i32 = 10;
const BEAM_ROW_SPEED: (i32, i32) = (10, 40);
const BEAM_COLUMN_SPEED: (i32, i32) = (6, 10);
const BEAM_GRADIENT_STEPS: &[usize] = &[2, 8];
const BEAM_GRADIENT_FRAMES: u32 = 2;
const FINAL_GRADIENT_STEPS: usize = 12;
const FADE_STEPS: usize = 10;
const FADE_FRAMES: u32 = 5;
const BRIGHTEN_FRAMES: u32 = 1;
const FINAL_WIPE_SPEED: usize = 1;
const FADE_BRIGHTNESS: f64 = 0.7;
const MAX_FRAMES: usize = 50_000;

pub struct Beams;

impl Beams {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Beams {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Beams {
    fn name(&self) -> &str {
        "beams"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_input(input, TerminalConfig::default());
        if terminal.character_count() == 0 {
            return Vec::new();
        }

        let beam_stops = [
            Color::rgb(0xff, 0xff, 0xff),
            Color::rgb(0x00, 0xd1, 0xff),
            Color::rgb(0x8a, 0x00, 0x8a),
        ];
        let final_stops = [
            Color::rgb(0x8a, 0x00, 0x8a),
            Color::rgb(0x00, 0xd1, 0xff),
            Color::rgb(0xff, 0xff, 0xff),
        ];

        let beam_colors = paired_spectrum(&beam_stops, BEAM_GRADIENT_STEPS);
        let beam_row = beam_sequence(BEAM_ROW_SYMBOLS, &beam_colors);
        let beam_col = beam_sequence(BEAM_COLUMN_SYMBOLS, &beam_colors);

        let final_gradient = Gradient::new(&final_stops, FINAL_GRADIENT_STEPS);
        let (min_row, max_row) = row_extents(&terminal);
        let denom = f64::from((max_row - min_row).max(1));

        let mut final_colors: HashMap<CharacterId, Color> = HashMap::new();
        let mut fade_colors: HashMap<CharacterId, Vec<Color>> = HashMap::new();
        for ch in terminal.get_characters() {
            let t = f64::from(ch.input_coord.row - min_row) / denom;
            let color = final_gradient
                .mapped_color(t)
                .unwrap_or(final_stops[0]);
            fade_colors.insert(ch.id, fade_spectrum(color));
            final_colors.insert(ch.id, color);
        }

        let mut rng = Rng::new(fnv1a(input.as_bytes()));
        let mut pending = build_groups(&terminal, &mut rng);
        let mut active_groups: Vec<Group> = Vec::new();
        let mut anims: HashMap<CharacterId, Anim> = HashMap::new();
        let mut wipe_groups: VecDeque<Vec<CharacterId>> = VecDeque::new();
        let mut delay = 0i32;
        let mut phase = Phase::Beams;
        let mut frames = Vec::new();

        while frames.len() < MAX_FRAMES {
            if phase == Phase::Beams {
                if pending.is_empty() && active_groups.is_empty() && anims.is_empty() {
                    phase = Phase::Wipe;
                    wipe_groups = diagonal_groups(&terminal);
                } else {
                    if delay == 0 {
                        if let Some(group) = pending.pop_front() {
                            active_groups.push(group);
                        }
                        delay = BEAM_DELAY;
                    } else {
                        delay -= 1;
                    }
                    for group in &mut active_groups {
                        group.next_character_counter += group.speed;
                        let emit = group.next_character_counter as i32;
                        if emit > 0 {
                            for _ in 0..emit {
                                if let Some(id) = group.take_next() {
                                    start_beam(
                                        &mut terminal,
                                        &mut anims,
                                        id,
                                        group.is_column,
                                        &beam_row,
                                        &beam_col,
                                    );
                                }
                            }
                        }
                        if group.chars.is_empty() {
                            group.hold_time -= 1;
                        }
                    }
                    active_groups.retain(|g| !g.complete());
                }
            }

            if phase == Phase::Wipe {
                if !wipe_groups.is_empty() {
                    for _ in 0..FINAL_WIPE_SPEED {
                        if let Some(group) = wipe_groups.pop_front() {
                            for id in group {
                                start_brighten(&mut terminal, &mut anims, &final_colors, id);
                            }
                        }
                    }
                }
                if wipe_groups.is_empty() && anims.is_empty() {
                    phase = Phase::Complete;
                }
            }

            if phase == Phase::Complete {
                break;
            }

            frames.push(terminal.render_frame());
            step_anims(
                &mut terminal,
                &mut anims,
                &final_colors,
                &fade_colors,
                &beam_row,
                &beam_col,
            );
        }

        frames
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Beams,
    Wipe,
    Complete,
}

#[derive(Clone, Copy)]
enum AnimPhase {
    Beam,
    Fade,
    Brighten,
}

struct Anim {
    phase: AnimPhase,
    index: usize,
    played: u32,
    use_column_glyphs: bool,
}

struct Group {
    chars: VecDeque<CharacterId>,
    is_column: bool,
    speed: f64,
    next_character_counter: f64,
    hold_time: i32,
}

impl Group {
    fn take_next(&mut self) -> Option<CharacterId> {
        self.next_character_counter -= 1.0;
        self.chars.pop_front()
    }

    fn complete(&self) -> bool {
        self.chars.is_empty() && self.hold_time <= 0
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }

    fn randint(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi - lo + 1) as u32;
        lo + (self.next_u32() % span.max(1)) as i32
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = (self.next_u32() as usize) % (i + 1);
            items.swap(i, j);
        }
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x00000100000001b3);
    }
    hash
}

fn paired_spectrum(stops: &[Color], steps: &[usize]) -> Vec<Color> {
    if stops.is_empty() {
        return Vec::new();
    }
    if stops.len() == 1 {
        return stops.to_vec();
    }
    let mut out = Vec::new();
    for (i, pair) in stops.windows(2).enumerate() {
        let n = steps.get(i).copied().or_else(|| steps.last().copied()).unwrap_or(1).max(1);
        let piece = Gradient::new(&[pair[0], pair[1]], n);
        let spec = piece.spectrum();
        if out.is_empty() {
            out.extend_from_slice(spec);
        } else if spec.len() > 1 {
            out.extend_from_slice(&spec[1..]);
        }
    }
    out
}

fn beam_sequence(symbols: &[&str], colors: &[Color]) -> Vec<(String, Color)> {
    let mut frames = Vec::new();
    if colors.is_empty() {
        return frames;
    }
    for symbol in symbols {
        for color in colors {
            frames.push(((*symbol).to_string(), *color));
        }
    }
    frames
}

fn fade_spectrum(final_c: Color) -> Vec<Color> {
    let dim = final_c.adjust_brightness(FADE_BRIGHTNESS);
    let spec = Gradient::new(&[final_c, dim], FADE_STEPS).spectrum().to_vec();
    if spec.is_empty() {
        vec![final_c]
    } else {
        spec
    }
}

fn row_extents(terminal: &Terminal) -> (i32, i32) {
    let mut min_row = i32::MAX;
    let mut max_row = i32::MIN;
    for ch in terminal.get_characters() {
        min_row = min_row.min(ch.input_coord.row);
        max_row = max_row.max(ch.input_coord.row);
    }
    if min_row > max_row {
        (0, 0)
    } else {
        (min_row, max_row)
    }
}

fn build_groups(terminal: &Terminal, rng: &mut Rng) -> VecDeque<Group> {
    let mut rows: BTreeMap<i32, Vec<CharacterId>> = BTreeMap::new();
    let mut cols: BTreeMap<i32, Vec<CharacterId>> = BTreeMap::new();
    for ch in terminal.get_characters() {
        rows.entry(ch.input_coord.row).or_default().push(ch.id);
        cols.entry(ch.input_coord.column).or_default().push(ch.id);
    }

    let mut groups = Vec::new();
    for row in rows.into_values() {
        let mut chars: VecDeque<CharacterId> = row.into();
        // ROW_BOTTOM_TO_TOP, then reversed so the beam travels right-to-left.
        let reversed: VecDeque<CharacterId> = chars.make_contiguous().iter().copied().rev().collect();
        chars = reversed;
        let len = chars.len() as i32;
        groups.push(Group {
            chars,
            is_column: false,
            speed: f64::from(rng.randint(BEAM_ROW_SPEED.0, BEAM_ROW_SPEED.1)) * 0.1,
            next_character_counter: 0.0,
            hold_time: len / 10,
        });
    }
    for col in cols.into_values() {
        let chars: VecDeque<CharacterId> = col.into();
        let len = chars.len() as i32;
        groups.push(Group {
            chars,
            is_column: true,
            speed: f64::from(rng.randint(BEAM_COLUMN_SPEED.0, BEAM_COLUMN_SPEED.1)) * 0.1,
            next_character_counter: 0.0,
            hold_time: len / 20,
        });
    }
    rng.shuffle(&mut groups);
    groups.into()
}

fn diagonal_groups(terminal: &Terminal) -> VecDeque<Vec<CharacterId>> {
    // DIAGONAL_TOP_LEFT_TO_BOTTOM_RIGHT: group by column - row, ascending.
    let mut diags: BTreeMap<i32, Vec<CharacterId>> = BTreeMap::new();
    for ch in terminal.get_characters() {
        let key = ch.input_coord.column - ch.input_coord.row;
        diags.entry(key).or_default().push(ch.id);
    }
    diags.into_values().collect()
}

fn paint(terminal: &mut Terminal, id: CharacterId, symbol: &str, color: Color, bold: bool) {
    if let Some(ch) = terminal.get_character_mut(id) {
        ch.animation.set_appearance(symbol, Some(ColorPair::fg(color)));
        ch.animation.current_character_visual.bold = bold;
        ch.animation.current_character_visual.refresh();
    }
}

fn paint_input(terminal: &mut Terminal, id: CharacterId, color: Color, bold: bool) {
    if let Some(ch) = terminal.get_character_mut(id) {
        let symbol = ch.input_symbol.clone();
        ch.animation.set_appearance(&symbol, Some(ColorPair::fg(color)));
        ch.animation.current_character_visual.bold = bold;
        ch.animation.current_character_visual.refresh();
    }
}

fn start_beam(
    terminal: &mut Terminal,
    anims: &mut HashMap<CharacterId, Anim>,
    id: CharacterId,
    column: bool,
    beam_row: &[(String, Color)],
    beam_col: &[(String, Color)],
) {
    anims.insert(
        id,
        Anim {
            phase: AnimPhase::Beam,
            index: 0,
            played: 0,
            use_column_glyphs: column,
        },
    );
    let seq = if column { beam_col } else { beam_row };
    if let Some((symbol, color)) = seq.first() {
        paint(terminal, id, symbol, *color, false);
    }
    terminal.set_character_visibility(id, true);
}

fn start_brighten(
    terminal: &mut Terminal,
    anims: &mut HashMap<CharacterId, Anim>,
    final_colors: &HashMap<CharacterId, Color>,
    id: CharacterId,
) {
    anims.insert(
        id,
        Anim {
            phase: AnimPhase::Brighten,
            index: 0,
            played: 0,
            use_column_glyphs: false,
        },
    );
    let color = final_colors
        .get(&id)
        .copied()
        .unwrap_or(Color::rgb(255, 255, 255));
    paint_input(terminal, id, color, true);
    terminal.set_character_visibility(id, true);
}

fn step_anims(
    terminal: &mut Terminal,
    anims: &mut HashMap<CharacterId, Anim>,
    final_colors: &HashMap<CharacterId, Color>,
    fade_colors: &HashMap<CharacterId, Vec<Color>>,
    beam_row: &[(String, Color)],
    beam_col: &[(String, Color)],
) {
    let ids: Vec<CharacterId> = anims.keys().copied().collect();
    let mut finished = Vec::new();
    for id in ids {
        let Some(anim) = anims.get_mut(&id) else {
            continue;
        };
        anim.played = anim.played.saturating_add(1);
        let (duration, len) = match anim.phase {
            AnimPhase::Beam => {
                let seq = if anim.use_column_glyphs {
                    beam_col
                } else {
                    beam_row
                };
                (BEAM_GRADIENT_FRAMES, seq.len())
            }
            AnimPhase::Fade => {
                let n = fade_colors.get(&id).map(Vec::len).unwrap_or(1);
                (FADE_FRAMES, n)
            }
            AnimPhase::Brighten => (BRIGHTEN_FRAMES, 1),
        };
        if anim.played < duration.max(1) {
            continue;
        }
        anim.played = 0;
        anim.index = anim.index.saturating_add(1);
        if anim.index >= len.max(1) {
            match anim.phase {
                AnimPhase::Beam => {
                    anim.phase = AnimPhase::Fade;
                    anim.index = 0;
                    if let Some(color) = fade_colors.get(&id).and_then(|s| s.first()).copied() {
                        paint_input(terminal, id, color, false);
                    } else if let Some(color) = final_colors.get(&id).copied() {
                        paint_input(terminal, id, color, false);
                    }
                }
                AnimPhase::Fade | AnimPhase::Brighten => finished.push(id),
            }
        } else {
            match anim.phase {
                AnimPhase::Beam => {
                    let seq = if anim.use_column_glyphs {
                        beam_col
                    } else {
                        beam_row
                    };
                    if let Some((symbol, color)) = seq.get(anim.index) {
                        paint(terminal, id, symbol, *color, false);
                    }
                }
                AnimPhase::Fade => {
                    if let Some(color) = fade_colors.get(&id).and_then(|s| s.get(anim.index)).copied()
                    {
                        paint_input(terminal, id, color, false);
                    }
                }
                AnimPhase::Brighten => {}
            }
        }
    }
    for id in finished {
        anims.remove(&id);
    }
}
