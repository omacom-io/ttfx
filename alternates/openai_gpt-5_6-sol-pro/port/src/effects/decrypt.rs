
use super::Effect;
use crate::engine::{CharacterVisual, Frame, Scene, Terminal};
use crate::utils::graphics::{Color, Style};

const TYPING_SPEED: usize = 2;
const CIPHERTEXT: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H',
    'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
    's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '!', '@', '#', '$', '%', '^', '&', '*', '+', '=',
    '?', '/', '\\', '|', ':', ';', '<', '>', '~',
];
const BLOCKS: &[char] = &['█', '▓', '▒', '░'];

pub struct Decrypt;

impl Decrypt {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Decrypt {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Decrypt {
    fn name(&self) -> &str {
        "decrypt"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);
        let mut rng = LocalRng::from_input(input);
        let mut frames = Vec::new();

        for character in terminal.characters_mut() {
            character.visible = false;
        }

        let character_count = terminal.characters().len();
        let mut typing_index = 0;

        while typing_index < character_count {
            let end = (typing_index + TYPING_SPEED).min(character_count);

            for character in &mut terminal.characters_mut()[typing_index..end] {
                let symbol = CIPHERTEXT[rng.index(CIPHERTEXT.len())];
                let color = ciphertext_color(rng.index(3));
                character.visible = true;
                character.set_appearance(symbol, Style::default().with_foreground(color));
            }

            typing_index = end;
            frames.push(terminal.render_frame());
        }

        let canvas_height = terminal.canvas().height();

        for character in terminal.characters_mut() {
            let mut scene = Scene::new(false);
            let cipher_frames = rng.range_inclusive(5, 10);

            for _ in 0..cipher_frames {
                let symbol = CIPHERTEXT[rng.index(CIPHERTEXT.len())];
                let color = ciphertext_color(rng.index(3));
                let duration = rng.range_inclusive(1, 2) as u32;
                scene.add_frame(Frame::new(
                    CharacterVisual::new(symbol, Style::default().with_foreground(color)),
                    duration,
                ));
            }

            for &symbol in BLOCKS {
                let color = ciphertext_color(rng.index(3));
                scene.add_frame(Frame::new(
                    CharacterVisual::new(symbol, Style::default().with_foreground(color)),
                    1,
                ));
            }

            let final_style =
                Style::default().with_foreground(final_color(character.position.y, canvas_height));
            scene.add_frame(Frame::new(
                CharacterVisual::new(character.input_symbol, final_style),
                1,
            ));

            character.animation.activate_scene(scene);
        }

        if character_count == 0 {
            frames.push(terminal.render_frame());
            return frames;
        }

        loop {
            terminal.step();
            frames.push(terminal.render_frame());

            let any_active = terminal.characters().iter().any(|character| {
                character
                    .animation
                    .active_scene()
                    .is_some_and(|scene| scene.is_active())
            });

            if !any_active {
                break;
            }
        }

        frames
    }
}

fn ciphertext_color(index: usize) -> Color {
    match index {
        0 => Color::rgb(0x00, 0x80, 0x00),
        1 => Color::rgb(0x00, 0xcb, 0x00),
        _ => Color::rgb(0x00, 0xff, 0x00),
    }
}

fn final_color(row: i32, canvas_height: usize) -> Color {
    let progress = if canvas_height <= 1 {
        0.0
    } else {
        (row.max(0) as f64 / (canvas_height - 1) as f64).clamp(0.0, 1.0)
    };

    let start = (0_u8, 0xd5_u8, 0_u8);
    let end = (0xff_u8, 0xff_u8, 0xff_u8);

    Color::rgb(
        interpolate(start.0, end.0, progress),
        interpolate(start.1, end.1, progress),
        interpolate(start.2, end.2, progress),
    )
}

fn interpolate(start: u8, end: u8, progress: f64) -> u8 {
    let value = start as f64 + (end as f64 - start as f64) * progress;
    value.round().clamp(0.0, 255.0) as u8
}

struct LocalRng {
    state: u64,
}

impl LocalRng {
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

    fn index(&mut self, length: usize) -> usize {
        if length <= 1 {
            0
        } else {
            (self.next_u64() % length as u64) as usize
        }
    }

    fn range_inclusive(&mut self, minimum: usize, maximum: usize) -> usize {
        if maximum <= minimum {
            minimum
        } else {
            minimum + self.index(maximum - minimum + 1)
        }
    }
}
