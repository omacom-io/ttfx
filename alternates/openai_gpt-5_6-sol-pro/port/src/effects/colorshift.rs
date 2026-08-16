
use super::Effect;
use crate::engine::{CharacterVisual, Frame, Scene, Terminal};
use crate::utils::{Color, Style};

const GRADIENT_STEPS: usize = 12;
const GRADIENT_FRAMES: u32 = 5;

#[derive(Debug, Clone, Copy, Default)]
pub struct Colorshift;

impl Colorshift {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Colorshift {
    fn name(&self) -> &str {
        "colorshift"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);

        if terminal.characters().is_empty() {
            return Vec::new();
        }

        let spectrum = build_spectrum();
        let canvas_width = terminal.canvas().width();
        let final_spectrum_index = spectrum.len().saturating_sub(1);

        for character in terminal.characters_mut() {
            let phase = if canvas_width <= 1 {
                0
            } else {
                let x = character
                    .position
                    .x
                    .clamp(0, canvas_width.saturating_sub(1) as i32)
                    as usize;

                x * final_spectrum_index / canvas_width.saturating_sub(1)
            };

            let mut scene = Scene::new(false);

            for offset in 0..spectrum.len() {
                let color = spectrum[(phase + offset) % spectrum.len()];
                let style = Style::default().with_foreground(color);

                scene.add_frame(Frame::new(
                    CharacterVisual::new(character.input_symbol, style),
                    GRADIENT_FRAMES,
                ));
            }

            character.animation.activate_scene(scene);
        }

        let mut frames = Vec::new();

        while terminal.characters().iter().any(|character| {
            character
                .animation
                .active_scene()
                .map_or(false, Scene::is_active)
        }) {
            terminal.step();
            frames.push(terminal.render_frame());
        }

        frames
    }
}

fn build_spectrum() -> Vec<Color> {
    const STOPS: [Color; 7] = [
        Color::rgb(0xe8, 0x14, 0x16),
        Color::rgb(0xff, 0xa5, 0x00),
        Color::rgb(0xfa, 0xeb, 0x36),
        Color::rgb(0x79, 0xc3, 0x14),
        Color::rgb(0x48, 0x7d, 0xe7),
        Color::rgb(0x4b, 0x36, 0x9d),
        Color::rgb(0x70, 0x36, 0x9d),
    ];

    let mut spectrum = Vec::with_capacity(STOPS.len() * GRADIENT_STEPS);

    for index in 0..STOPS.len() {
        let start = STOPS[index];
        let end = STOPS[(index + 1) % STOPS.len()];

        for step in 0..GRADIENT_STEPS {
            let progress = step as f64 / GRADIENT_STEPS as f64;
            spectrum.push(interpolate_color(start, end, progress));
        }
    }

    spectrum
}

fn interpolate_color(start: Color, end: Color, progress: f64) -> Color {
    let (
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
    ) = (start, end)
    else {
        return start;
    };

    Color::rgb(
        interpolate_channel(start_r, end_r, progress),
        interpolate_channel(start_g, end_g, progress),
        interpolate_channel(start_b, end_b, progress),
    )
}

fn interpolate_channel(start: u8, end: u8, progress: f64) -> u8 {
    let value = f64::from(start) + (f64::from(end) - f64::from(start)) * progress;
    value.round().clamp(0.0, 255.0) as u8
}
