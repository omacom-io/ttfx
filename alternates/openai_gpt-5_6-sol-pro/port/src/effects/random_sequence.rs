
use std::time::{SystemTime, UNIX_EPOCH};

use super::Effect;
use crate::engine::{CharacterId, CharacterVisual, Frame, Scene, Terminal};
use crate::utils::graphics::{Color, Style};

const REVEAL_SPEED: f64 = 0.004;
const FADE_STEPS: usize = 10;
const FADE_FRAME_DURATION: u32 = 5;

const STARTING_COLOR: Color = Color::rgb(0x00, 0x00, 0x00);
const FINAL_GRADIENT_STOPS: [Color; 3] = [
    Color::rgb(0x8a, 0x00, 0x8a),
    Color::rgb(0x00, 0xd1, 0xff),
    Color::rgb(0xff, 0xff, 0xff),
];

pub struct RandomSequence;

impl RandomSequence {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RandomSequence {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for RandomSequence {
    fn name(&self) -> &str {
        "random_sequence"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);

        if terminal.characters().is_empty() {
            return Vec::new();
        }

        let width = terminal.canvas().width();
        let height = terminal.canvas().height();
        let starting_style = Style::default().with_foreground(STARTING_COLOR);

        let mut pending: Vec<CharacterId> = terminal
            .characters()
            .iter()
            .map(|character| character.id)
            .collect();

        for character in terminal.characters_mut() {
            character.visible = false;
            character.set_appearance(character.input_symbol, starting_style.clone());
        }

        shuffle(&mut pending, seed_for(input));

        let mut active = Vec::new();
        let mut frames = Vec::new();

        while !pending.is_empty() || !active.is_empty() {
            if !pending.is_empty() {
                let activation_count = ((pending.len() as f64 * REVEAL_SPEED) as usize)
                    .max(1)
                    .min(pending.len());

                for _ in 0..activation_count {
                    let Some(id) = pending.pop() else {
                        break;
                    };

                    let Some(character) = terminal.character_mut(id) else {
                        continue;
                    };

                    let final_color =
                        final_color_for(character.position.x, character.position.y, width, height);
                    let scene = fade_scene(character.input_symbol, final_color);

                    character.visible = true;
                    character.animation.activate_scene(scene);
                    active.push(id);
                }
            }

            terminal.step();

            active.retain(|id| {
                terminal
                    .character(*id)
                    .and_then(|character| character.animation.active_scene())
                    .is_some_and(|scene| scene.is_active())
            });

            frames.push(terminal.render_frame());
        }

        frames
    }
}

fn fade_scene(symbol: char, final_color: Color) -> Scene {
    let mut scene = Scene::new(false);

    for step in 0..FADE_STEPS {
        let progress = if FADE_STEPS <= 1 {
            1.0
        } else {
            step as f64 / (FADE_STEPS - 1) as f64
        };

        let color = interpolate_color(STARTING_COLOR, final_color, progress);
        let style = Style::default().with_foreground(color);

        scene.add_frame(Frame::new(
            CharacterVisual::new(symbol, style),
            FADE_FRAME_DURATION,
        ));
    }

    scene
}

fn final_color_for(x: i32, y: i32, width: usize, height: usize) -> Color {
    let horizontal = x.max(0) as f64;
    let vertical = (height as i32 - 1 - y).max(0) as f64;
    let denominator = (width.saturating_sub(1) + height.saturating_sub(1)) as f64;

    let progress = if denominator <= f64::EPSILON {
        0.0
    } else {
        ((horizontal + vertical) / denominator).clamp(0.0, 1.0)
    };

    multi_stop_color(&FINAL_GRADIENT_STOPS, progress)
}

fn multi_stop_color(stops: &[Color], progress: f64) -> Color {
    if stops.is_empty() {
        return Color::rgb(0, 0, 0);
    }

    if stops.len() == 1 {
        return stops[0];
    }

    let scaled = progress.clamp(0.0, 1.0) * (stops.len() - 1) as f64;
    let segment = (scaled.floor() as usize).min(stops.len() - 2);
    let local_progress = scaled - segment as f64;

    interpolate_color(stops[segment], stops[segment + 1], local_progress)
}

fn interpolate_color(start: Color, end: Color, progress: f64) -> Color {
    let (start_r, start_g, start_b) = rgb_components(start);
    let (end_r, end_g, end_b) = rgb_components(end);
    let progress = progress.clamp(0.0, 1.0);

    Color::rgb(
        interpolate_channel(start_r, end_r, progress),
        interpolate_channel(start_g, end_g, progress),
        interpolate_channel(start_b, end_b, progress),
    )
}

fn rgb_components(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb { r, g, b } => (r, g, b),
        Color::Ansi(value) => (value, value, value),
    }
}

fn interpolate_channel(start: u8, end: u8, progress: f64) -> u8 {
    let value = start as f64 + (end as f64 - start as f64) * progress;
    value.round().clamp(0.0, 255.0) as u8
}

fn seed_for(input: &str) -> u64 {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);

    let mut hash = 0xcbf2_9ce4_8422_2325_u64;

    for byte in input.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    let seed = timestamp ^ hash;
    if seed == 0 {
        0x9e37_79b9_7f4a_7c15
    } else {
        seed
    }
}

fn shuffle<T>(values: &mut [T], mut state: u64) {
    if values.len() < 2 {
        return;
    }

    for index in (1..values.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;

        let other = (state as usize) % (index + 1);
        values.swap(index, other);
    }
}
