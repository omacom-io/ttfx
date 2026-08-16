
use super::Effect;
use crate::engine::{CharacterVisual, Frame, Scene, Terminal};
use crate::utils::graphics::{Color, Style};

const WAVE_SYMBOLS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
const WAVE_COUNT: usize = 3;
const WAVE_FRAME_DURATION: u32 = 2;

const GRADIENT_START: Color = Color::rgb(0xf0, 0xff, 0x65);
const GRADIENT_END: Color = Color::rgb(0x65, 0xc6, 0xff);

#[derive(Debug, Clone, Copy, Default)]
pub struct Waves;

impl Waves {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Waves {
    fn name(&self) -> &str {
        "waves"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);

        if terminal.characters().is_empty() {
            return vec![terminal.render_frame()];
        }

        let height = terminal.canvas().height();
        let mut longest_delay = 0_u32;

        for character in terminal.characters_mut() {
            let delay = u32::try_from(character.position.x.max(0)).unwrap_or(u32::MAX);
            longest_delay = longest_delay.max(delay);

            let mut scene = Scene::new(false);

            if delay > 0 {
                scene.add_frame(Frame::new(
                    CharacterVisual::new(character.input_symbol, Style::default()),
                    delay,
                ));
            }

            for _ in 0..WAVE_COUNT {
                for (index, symbol) in WAVE_SYMBOLS.iter().copied().enumerate() {
                    let progress = if WAVE_SYMBOLS.len() <= 1 {
                        1.0
                    } else {
                        index as f64 / (WAVE_SYMBOLS.len() - 1) as f64
                    };

                    let style =
                        Style::default().with_foreground(interpolate_color(
                            GRADIENT_START,
                            GRADIENT_END,
                            progress,
                        ));

                    scene.add_frame(Frame::new(
                        CharacterVisual::new(symbol, style),
                        WAVE_FRAME_DURATION,
                    ));
                }
            }

            let row_progress = if height <= 1 {
                0.0
            } else {
                character.position.y.max(0) as f64 / (height - 1) as f64
            };

            let final_style =
                Style::default().with_foreground(interpolate_color(
                    GRADIENT_START,
                    GRADIENT_END,
                    row_progress,
                ));

            scene.add_frame(Frame::new(
                CharacterVisual::new(character.input_symbol, final_style),
                1,
            ));

            character.visible = true;
            character.animation.activate_scene(scene);
        }

        let wave_duration = WAVE_COUNT
            .saturating_mul(WAVE_SYMBOLS.len())
            .saturating_mul(WAVE_FRAME_DURATION as usize);
        let maximum_steps = usize::try_from(longest_delay)
            .unwrap_or(usize::MAX)
            .saturating_add(wave_duration)
            .saturating_add(2);

        let mut frames = Vec::with_capacity(maximum_steps);

        for _ in 0..maximum_steps {
            terminal.step();
            frames.push(terminal.render_frame());

            let finished = terminal.characters().iter().all(|character| {
                character
                    .animation
                    .active_scene()
                    .map_or(true, Scene::is_finished)
            });

            if finished {
                break;
            }
        }

        if frames.is_empty() {
            frames.push(terminal.render_frame());
        }

        frames
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
        (Color::Ansi(start_value), Color::Ansi(end_value)) => Color::ansi(
            interpolate_channel(start_value, end_value, progress),
        ),
        (_, end) => {
            if progress < 0.5 {
                start
            } else {
                end
            }
        }
    }
}

fn interpolate_channel(start: u8, end: u8, progress: f64) -> u8 {
    let value = f64::from(start) + (f64::from(end) - f64::from(start)) * progress;
    value.round().clamp(0.0, 255.0) as u8
}
