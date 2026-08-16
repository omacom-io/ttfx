use super::Effect;
use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, ColorPair, Gradient};

pub struct RandomSequence;

impl RandomSequence {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for RandomSequence {
    fn name(&self) -> &str {
        "random_sequence"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (width, height) = Terminal::autodetect_size();
        let mut terminal = Terminal::from_input(input, width, height);
        terminal.clear_canvas();

        let char_count = terminal.characters.len();
        if char_count == 0 {
            return vec![terminal.write_frame()];
        }

        let mut ids: Vec<u32> = terminal.characters.iter().map(|c| c.id).collect();
        shuffled(&mut ids, hash_input(input));

        let gradient = Gradient::new()
            .add_stop(0.0, Color::CYAN)
            .add_stop(0.5, Color::BLUE)
            .add_stop(1.0, Color::MAGENTA);

        let color_by_id: Vec<(u32, Color)> = terminal
            .characters
            .iter()
            .enumerate()
            .map(|(index, character)| {
                let t = if char_count > 1 {
                    index as f32 / (char_count - 1) as f32
                } else {
                    0.0
                };
                (character.id, gradient.color_at(t))
            })
            .collect();

        let mut frames = Vec::with_capacity(char_count);

        for id in ids {
            let fg = color_by_id
                .iter()
                .find(|(cid, _)| *cid == id)
                .map(|(_, color)| *color)
                .unwrap_or(Color::WHITE);
            let pair = ColorPair::new(fg, Color::BLACK);

            if let Some(character) = terminal.characters.iter_mut().find(|c| c.id == id) {
                character.output_symbol = character.input_symbol.clone();
                character.style = CellStyle::with_color_pair(pair);
                character.visible = true;

                let x = character.position.x.round() as u16;
                let y = character.position.y.round() as u16;
                terminal.canvas.set_cell(
                    x,
                    y,
                    Cell::new(character.output_symbol.clone(), character.style),
                );
            }

            frames.push(terminal.write_frame());
        }

        frames
    }
}

fn hash_input(input: &str) -> u64 {
    let mut seed = 0xcbf29ce484222325u64;
    for byte in input.as_bytes() {
        seed ^= *byte as u64;
        seed = seed.wrapping_mul(0x100000001b3);
    }
    seed
}

fn next_random(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x.wrapping_shl(13);
    x ^= x.wrapping_shr(7);
    x ^= x.wrapping_shl(17);
    *state = x;
    x
}

fn shuffled(items: &mut [u32], seed: u64) {
    let mut rng = seed;
    if rng == 0 {
        rng = 0x9e3779b97f4a7c15;
    }

    for i in (1..items.len()).rev() {
        let j = (next_random(&mut rng) as usize) % (i + 1);
        items.swap(i, j);
    }
}
