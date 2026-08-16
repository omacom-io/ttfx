use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{find_length_of_line, lerp_coord, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

const CENTER_MOVEMENT_SPEED: f64 = 0.35;
const FULL_MOVEMENT_SPEED: f64 = 0.35;
const FINAL_GRADIENT_STEPS: usize = 12;
const FINAL_GRADIENT_FRAMES: usize = 5;
const FULL_EXP_GRADIENT_STEPS: usize = 10;
const MAX_FRAMES: usize = 10_000;

fn in_out_sine(t: f64) -> f64 {
    (1.0 - (t * std::f64::consts::PI).cos()) / 2.0
}

fn hex_color(hex: &str, fallback: (u8, u8, u8)) -> Color {
    Color::from_hex(hex).unwrap_or(Color::rgb(fallback.0, fallback.1, fallback.2))
}

enum Phase {
    Centering,
    Expanding,
}

struct CharState {
    center_start: Coord,
    center_end: Coord,
    full_end: Coord,
    center_len: f64,
    full_len: f64,
    center_progress: f64,
    full_progress: f64,
    anim_frame: usize,
    colors: Vec<Color>,
}

pub struct Middleout;

impl Middleout {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Middleout {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Middleout {
    fn name(&self) -> &str {
        "middleout"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_input(input, TerminalConfig::default());
        if terminal.character_count() == 0 {
            return Vec::new();
        }

        let starting_color = hex_color("ffffff", (255, 255, 255));
        let final_gradient = Gradient::new(
            &[
                hex_color("8A008A", (0x8A, 0x00, 0x8A)),
                hex_color("00D1FF", (0x00, 0xD1, 0xFF)),
                hex_color("FFFFFF", (255, 255, 255)),
            ],
            FINAL_GRADIENT_STEPS,
        );

        let canvas_center = terminal.canvas.center();
        let (text_bottom, text_top) = {
            let chars = terminal.get_characters();
            let bottom = chars
                .iter()
                .map(|ch| ch.input_coord.row)
                .min()
                .unwrap_or(canvas_center.row);
            let top = chars
                .iter()
                .map(|ch| ch.input_coord.row)
                .max()
                .unwrap_or(canvas_center.row);
            (bottom, top)
        };

        let mut states: Vec<CharState> = terminal
            .get_characters()
            .iter()
            .map(|ch| {
                let input_coord = ch.input_coord;
                // Default expand-direction is vertical: collapse onto the center row.
                let center_end = Coord::new(input_coord.column, canvas_center.row);
                let progress = if text_top == text_bottom {
                    0.0
                } else {
                    f64::from(input_coord.row - text_bottom)
                        / f64::from(text_top - text_bottom)
                };
                let final_color = final_gradient
                    .mapped_color(progress)
                    .unwrap_or(starting_color);
                let mut colors = Gradient::new(
                    &[starting_color, final_color],
                    FULL_EXP_GRADIENT_STEPS,
                )
                .spectrum()
                .to_vec();
                if colors.is_empty() {
                    colors.push(starting_color);
                }
                CharState {
                    center_start: canvas_center,
                    center_end,
                    full_end: input_coord,
                    center_len: find_length_of_line(canvas_center, center_end),
                    full_len: find_length_of_line(center_end, input_coord),
                    center_progress: 0.0,
                    full_progress: 0.0,
                    anim_frame: 0,
                    colors,
                }
            })
            .collect();

        for ch in terminal.get_characters_mut() {
            ch.motion.current_coord = canvas_center;
            ch.is_visible = true;
        }

        let mut frames = Vec::new();
        let mut phase = Phase::Centering;

        loop {
            let mut any_active = false;

            for (ch, state) in terminal
                .get_characters_mut()
                .iter_mut()
                .zip(states.iter_mut())
            {
                let total_anim = state.colors.len() * FINAL_GRADIENT_FRAMES;
                let anim_active = state.anim_frame < total_anim;
                let color_idx = (state.anim_frame / FINAL_GRADIENT_FRAMES)
                    .min(state.colors.len().saturating_sub(1));
                let color = state.colors[color_idx];

                match phase {
                    Phase::Centering => {
                        if state.center_progress < 1.0 {
                            any_active = true;
                            if state.center_len <= 0.0 {
                                state.center_progress = 1.0;
                            } else {
                                state.center_progress = (state.center_progress
                                    + CENTER_MOVEMENT_SPEED / state.center_len)
                                    .min(1.0);
                            }
                        }
                        let t = in_out_sine(state.center_progress);
                        ch.motion.current_coord =
                            lerp_coord(state.center_start, state.center_end, t);
                    }
                    Phase::Expanding => {
                        if state.full_progress < 1.0 {
                            any_active = true;
                            if state.full_len <= 0.0 {
                                state.full_progress = 1.0;
                            } else {
                                state.full_progress = (state.full_progress
                                    + FULL_MOVEMENT_SPEED / state.full_len)
                                    .min(1.0);
                            }
                        }
                        let t = in_out_sine(state.full_progress);
                        ch.motion.current_coord =
                            lerp_coord(state.center_end, state.full_end, t);
                    }
                }

                if anim_active {
                    any_active = true;
                    state.anim_frame += 1;
                }

                let symbol = ch.input_symbol.clone();
                ch.animation
                    .set_appearance(&symbol, Some(ColorPair::fg(color)));
            }

            frames.push(terminal.render_frame());

            if !any_active {
                match phase {
                    Phase::Centering => phase = Phase::Expanding,
                    Phase::Expanding => break,
                }
            }

            if frames.len() >= MAX_FRAMES {
                break;
            }
        }

        frames
    }
}
