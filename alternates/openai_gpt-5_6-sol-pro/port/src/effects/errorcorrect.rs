
use std::collections::{BTreeMap, HashSet};

use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::Terminal;
use crate::utils::easing::out_quad;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Style};

const ERROR_PAIR_RATIO: f64 = 0.1;
const ERROR_HOLD_FRAMES: usize = 10;
const SWAP_FRAMES: usize = 6;
const CORRECT_HOLD_FRAMES: usize = 4;

const ERROR_COLOR: Color = Color::rgb(255, 0, 0);
const CORRECT_COLOR: Color = Color::rgb(0, 255, 0);

#[derive(Debug, Clone, Copy)]
struct ErrorPair {
    left_id: CharacterId,
    right_id: CharacterId,
    left_coord: Coord,
    right_coord: Coord,
}

#[derive(Debug, Clone, Copy)]
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn from_input(input: &str) -> Self {
        let mut state = 0xcbf2_9ce4_8422_2325_u64;

        for byte in input.bytes() {
            state ^= u64::from(byte);
            state = state.wrapping_mul(0x0000_0100_0000_01b3);
        }

        if state == 0 {
            state = 0x9e37_79b9_7f4a_7c15;
        }

        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for index in (1..values.len()).rev() {
            let other = (self.next_u64() as usize) % (index + 1);
            values.swap(index, other);
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Errorcorrect;

impl Errorcorrect {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Errorcorrect {
    fn name(&self) -> &str {
        "errorcorrect"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);

        if terminal.characters().is_empty() {
            return vec![terminal.render_frame()];
        }

        let mut pairs = build_error_pairs(&terminal, input);

        if pairs.is_empty() {
            return vec![terminal.render_frame()];
        }

        let error_style = Style::default().with_foreground(ERROR_COLOR);
        let correct_style = Style::default().with_foreground(CORRECT_COLOR);

        for pair in &pairs {
            set_character(
                &mut terminal,
                pair.left_id,
                pair.right_coord,
                error_style.clone(),
            );
            set_character(
                &mut terminal,
                pair.right_id,
                pair.left_coord,
                error_style.clone(),
            );
        }

        let mut frames = Vec::new();

        push_repeated_frame(&mut terminal, &mut frames, ERROR_HOLD_FRAMES);

        for pair in pairs.drain(..) {
            for step in 1..=SWAP_FRAMES {
                let raw_progress = step as f64 / SWAP_FRAMES as f64;
                let progress = out_quad(raw_progress);

                let left_position = pair.right_coord.lerp(pair.left_coord, progress);
                let right_position = pair.left_coord.lerp(pair.right_coord, progress);

                set_character(
                    &mut terminal,
                    pair.left_id,
                    left_position,
                    error_style.clone(),
                );
                set_character(
                    &mut terminal,
                    pair.right_id,
                    right_position,
                    error_style.clone(),
                );

                frames.push(terminal.render_frame());
            }

            set_character(
                &mut terminal,
                pair.left_id,
                pair.left_coord,
                correct_style.clone(),
            );
            set_character(
                &mut terminal,
                pair.right_id,
                pair.right_coord,
                correct_style.clone(),
            );

            push_repeated_frame(&mut terminal, &mut frames, CORRECT_HOLD_FRAMES);

            set_character(
                &mut terminal,
                pair.left_id,
                pair.left_coord,
                Style::default(),
            );
            set_character(
                &mut terminal,
                pair.right_id,
                pair.right_coord,
                Style::default(),
            );

            frames.push(terminal.render_frame());
        }

        if frames.is_empty() {
            frames.push(terminal.render_frame());
        }

        frames
    }
}

fn build_error_pairs(terminal: &Terminal, input: &str) -> Vec<ErrorPair> {
    let mut rows: BTreeMap<i32, Vec<(CharacterId, Coord)>> = BTreeMap::new();
    let mut eligible_count = 0_usize;

    for character in terminal.characters() {
        if character.input_symbol.is_whitespace() {
            continue;
        }

        eligible_count += 1;
        rows.entry(character.position.y)
            .or_default()
            .push((character.id, character.position));
    }

    let mut candidates = Vec::new();

    for row in rows.values_mut() {
        row.sort_by_key(|(_, coord)| coord.x);

        for adjacent in row.windows(2) {
            let (left_id, left_coord) = adjacent[0];
            let (right_id, right_coord) = adjacent[1];

            if right_coord.x == left_coord.x + 1 {
                candidates.push(ErrorPair {
                    left_id,
                    right_id,
                    left_coord,
                    right_coord,
                });
            }
        }
    }

    if candidates.is_empty() {
        return Vec::new();
    }

    let requested_pairs = ((eligible_count as f64 * ERROR_PAIR_RATIO).round() as usize)
        .max(1)
        .min(candidates.len());

    let mut rng = SimpleRng::from_input(input);
    rng.shuffle(&mut candidates);

    let mut selected = Vec::with_capacity(requested_pairs);
    let mut used = HashSet::new();

    for pair in candidates {
        if selected.len() >= requested_pairs {
            break;
        }

        if used.contains(&pair.left_id) || used.contains(&pair.right_id) {
            continue;
        }

        used.insert(pair.left_id);
        used.insert(pair.right_id);
        selected.push(pair);
    }

    rng.shuffle(&mut selected);
    selected
}

fn set_character(
    terminal: &mut Terminal,
    id: CharacterId,
    position: Coord,
    style: Style,
) {
    if let Some(character) = terminal.character_mut(id) {
        character.set_position(position);
        character.set_appearance(character.input_symbol, style);
    }
}

fn push_repeated_frame(
    terminal: &mut Terminal,
    frames: &mut Vec<String>,
    count: usize,
) {
    if count == 0 {
        return;
    }

    let frame = terminal.render_frame();
    frames.extend(std::iter::repeat(frame).take(count));
}
