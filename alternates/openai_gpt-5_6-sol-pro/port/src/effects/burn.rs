
use std::collections::BTreeMap;

use super::Effect;
use crate::engine::animation::{CharacterVisual, Frame, Scene};
use crate::engine::character::{CharacterId, EffectCharacter};
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, Style};

const STARTING_COLOR: Color = Color::rgb(0x83, 0x73, 0x73);
const BURN_COLORS: [Color; 4] = [
    Color::rgb(0xff, 0xff, 0xff),
    Color::rgb(0xff, 0xf7, 0x5d),
    Color::rgb(0xfe, 0x65, 0x0d),
    Color::rgb(0x8a, 0x00, 0x3c),
];
const FINAL_GRADIENT_START: Color = Color::rgb(0x00, 0xc3, 0xff);
const FINAL_GRADIENT_END: Color = Color::rgb(0xff, 0xff, 0x1c);
const FINAL_GRADIENT_STEPS: usize = 12;
const BURN_DELAY: usize = 2;
const FLAME_SYMBOLS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

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
        let mut terminal = Terminal::from_text(input);

        if terminal.characters().is_empty() {
            return Vec::new();
        }

        let canvas_height = terminal.canvas().height();
        let mut rows: BTreeMap<i32, Vec<CharacterId>> = BTreeMap::new();

        for character in terminal.characters_mut() {
            rows.entry(character.position.y)
                .or_default()
                .push(character.id);

            prepare_character(character, canvas_height);
        }

        let pending_rows: Vec<Vec<CharacterId>> = rows.into_iter().rev().map(|(_, ids)| ids).collect();
        let mut next_row = 0;
        let mut active = Vec::<CharacterId>::new();
        let mut delay_remaining = 0;
        let mut frames = Vec::new();

        while next_row < pending_rows.len() || !active.is_empty() {
            if next_row < pending_rows.len() {
                if delay_remaining == 0 {
                    for &id in &pending_rows[next_row] {
                        if let Some(character) = terminal.character_mut(id) {
                            if let Some(scene) = burn_scene(character, canvas_height) {
                                character.animation.activate_scene(scene);
                                active.push(id);
                            }
                        }
                    }

                    next_row += 1;
                    delay_remaining = BURN_DELAY;
                } else {
                    delay_remaining -= 1;
                }
            }

            terminal.step();

            active.retain(|&id| {
                terminal
                    .character(id)
                    .and_then(|character| character.animation.active_scene())
                    .is_some_and(|scene| !scene.is_finished())
            });

            frames.push(terminal.render_frame());
        }

        frames
    }
}

fn prepare_character(character: &mut EffectCharacter, canvas_height: usize) {
    let style = Style::default().with_foreground(STARTING_COLOR);
    character.set_appearance(character.input_symbol, style);
    character.visible = true;

    let _ = canvas_height;
}

fn burn_scene(character: &EffectCharacter, canvas_height: usize) -> Option<Scene> {
    let mut frames = Vec::with_capacity(BURN_COLORS.len() + 1);

    for (index, color) in BURN_COLORS.iter().copied().enumerate() {
        let symbol_index = (character.id.0 as usize)
            .wrapping_mul(5)
            .wrapping_add(index * 3)
            % FLAME_SYMBOLS.len();

        frames.push(Frame::new(
            CharacterVisual::new(
                FLAME_SYMBOLS[symbol_index],
                Style::default().with_foreground(color),
            ),
            2,
        ));
    }

    frames.push(Frame::new(
        CharacterVisual::new(
            character.input_symbol,
            Style::default().with_foreground(final_color(
                character.position.y,
                canvas_height,
            )),
        ),
        1,
    ));

    if frames.is_empty() {
        None
    } else {
        Some(Scene::with_frames(frames, false))
    }
}

fn final_color(row: i32, canvas_height: usize) -> Color {
    if canvas_height <= 1 {
        return FINAL_GRADIENT_START;
    }

    let maximum_row = (canvas_height - 1) as f64;
    let row = f64::from(row).clamp(0.0, maximum_row);

    // The original canvas coordinates increase upward. Rust canvas coordinates
    // increase downward, so invert the ratio to retain the original gradient.
    let vertical_ratio = 1.0 - row / maximum_row;
    let step = (vertical_ratio * (FINAL_GRADIENT_STEPS - 1) as f64).round();
    let ratio = step / (FINAL_GRADIENT_STEPS - 1) as f64;

    interpolate_color(FINAL_GRADIENT_START, FINAL_GRADIENT_END, ratio)
}

fn interpolate_color(start: Color, end: Color, ratio: f64) -> Color {
    let ratio = ratio.clamp(0.0, 1.0);

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
            interpolate_channel(start_r, end_r, ratio),
            interpolate_channel(start_g, end_g, ratio),
            interpolate_channel(start_b, end_b, ratio),
        ),
        _ if ratio < 0.5 => start,
        _ => end,
    }
}

fn interpolate_channel(start: u8, end: u8, ratio: f64) -> u8 {
    (f64::from(start) + (f64::from(end) - f64::from(start)) * ratio).round() as u8
}
