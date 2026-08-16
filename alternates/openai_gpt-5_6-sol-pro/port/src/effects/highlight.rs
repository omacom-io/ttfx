
use std::collections::{BTreeMap, BTreeSet};

use super::Effect;
use crate::engine::{CharacterId, CharacterVisual, Frame, Scene, Terminal};
use crate::utils::{Color, Style};

const HIGHLIGHT_DURATION: u32 = 3;
const FINAL_GRADIENT_STEPS: f64 = 24.0;

const GRADIENT_START: Color = Color::rgb(0x8a, 0x00, 0x8a);
const GRADIENT_MIDDLE: Color = Color::rgb(0x00, 0xd1, 0xff);
const GRADIENT_END: Color = Color::rgb(0xff, 0xff, 0xff);
const HIGHLIGHT_COLOR: Color = Color::rgb(0xff, 0xff, 0xff);

#[derive(Debug, Clone, Copy, Default)]
pub struct Highlight;

impl Highlight {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Highlight {
    fn name(&self) -> &str {
        "highlight"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);

        if terminal.characters().is_empty() {
            return Vec::new();
        }

        let width = terminal.canvas().width();
        let height = terminal.canvas().height();

        let mut groups: BTreeMap<i32, Vec<CharacterId>> = BTreeMap::new();

        for character in terminal.characters_mut() {
            let x_progress = axis_progress(character.position.x, width);
            let y_progress = if height <= 1 {
                0.0
            } else {
                1.0 - axis_progress(character.position.y, height)
            };

            let diagonal_progress = if width <= 1 {
                y_progress
            } else if height <= 1 {
                x_progress
            } else {
                (x_progress + y_progress) / 2.0
            };

            let final_color = final_gradient(diagonal_progress);
            let final_style = Style::default().with_foreground(final_color);
            let highlight_style = Style::default().with_foreground(HIGHLIGHT_COLOR);

            character.set_style(final_style.clone());

            let scene = Scene::with_frames(
                vec![
                    Frame::new(
                        CharacterVisual::new(character.input_symbol, highlight_style),
                        HIGHLIGHT_DURATION,
                    ),
                    Frame::new(
                        CharacterVisual::new(character.input_symbol, final_style),
                        1,
                    ),
                ],
                false,
            );

            character.animation.activate_scene(scene);
            character.animation.deactivate();

            groups
                .entry(character.position.x)
                .or_default()
                .push(character.id);
        }

        let groups: Vec<Vec<CharacterId>> = groups.into_values().collect();
        let mut next_group = 0;
        let mut active = BTreeSet::new();
        let mut frames = Vec::new();

        while next_group < groups.len() || !active.is_empty() {
            if let Some(group) = groups.get(next_group) {
                for &id in group {
                    if let Some(character) = terminal.character_mut(id) {
                        let final_style = character.style.clone();
                        let highlight_style =
                            Style::default().with_foreground(HIGHLIGHT_COLOR);

                        let scene = Scene::with_frames(
                            vec![
                                Frame::new(
                                    CharacterVisual::new(
                                        character.input_symbol,
                                        highlight_style,
                                    ),
                                    HIGHLIGHT_DURATION,
                                ),
                                Frame::new(
                                    CharacterVisual::new(
                                        character.input_symbol,
                                        final_style,
                                    ),
                                    1,
                                ),
                            ],
                            false,
                        );

                        if character.animation.activate_scene(scene) {
                            active.insert(id);
                        }
                    }
                }

                next_group += 1;
            }

            terminal.step();

            active.retain(|id| {
                terminal
                    .character(*id)
                    .and_then(|character| character.animation.active_scene())
                    .is_some_and(|scene| !scene.is_finished())
            });

            frames.push(terminal.render_frame());
        }

        frames
    }
}

fn axis_progress(position: i32, size: usize) -> f64 {
    if size <= 1 {
        0.0
    } else {
        (position.max(0) as f64 / (size - 1) as f64).clamp(0.0, 1.0)
    }
}

fn final_gradient(progress: f64) -> Color {
    let progress = progress.clamp(0.0, 1.0);
    let quantized = (progress * FINAL_GRADIENT_STEPS).round() / FINAL_GRADIENT_STEPS;

    if quantized <= 0.5 {
        interpolate_color(GRADIENT_START, GRADIENT_MIDDLE, quantized * 2.0)
    } else {
        interpolate_color(
            GRADIENT_MIDDLE,
            GRADIENT_END,
            (quantized - 0.5) * 2.0,
        )
    }
}

fn interpolate_color(start: Color, end: Color, progress: f64) -> Color {
    let progress = progress.clamp(0.0, 1.0);

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
        _ if progress < 0.5 => start,
        _ => end,
    }
}

fn interpolate_channel(start: u8, end: u8, progress: f64) -> u8 {
    (f64::from(start) + (f64::from(end) - f64::from(start)) * progress)
        .round()
        .clamp(0.0, 255.0) as u8
}
