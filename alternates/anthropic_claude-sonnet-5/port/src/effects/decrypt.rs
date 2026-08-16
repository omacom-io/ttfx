use super::Effect;
use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, ColorPair};

/// Simple xorshift64 PRNG, used only to pick flicker symbols. Not intended
/// to match upstream's `random` module bit-for-bit; deterministic within a
/// single run is all that's required here.
fn next_rand(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Port of `terminaltexteffects/effects/effect_decrypt.py`.
///
/// Every character starts out showing a short flicker of random
/// "ciphertext" symbols in a dim color, then characters are resolved to
/// their real symbol in input order, `typing_speed` characters per tick,
/// switching to a brighter "plaintext" color once resolved.
pub struct Decrypt {
    typing_speed: usize,
    flicker_ticks: usize,
    frames_per_flicker_symbol: u32,
    ciphertext_color: Color,
    plaintext_color: Color,
    symbol_set: Vec<char>,
}

impl Decrypt {
    pub fn new() -> Self {
        Decrypt {
            typing_speed: 1,
            flicker_ticks: 10,
            frames_per_flicker_symbol: 3,
            ciphertext_color: Color::Rgb(0, 128, 0),
            plaintext_color: Color::Rgb(0, 255, 0),
            symbol_set: "!@#$%^&*()_+-=[]{}|;:,.<>?/~`0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"
                .chars()
                .collect(),
        }
    }
}

impl Effect for Decrypt {
    fn name(&self) -> &str {
        "decrypt"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let mut rng_state: u64 = 0x2545_F491_4F6C_DD1D;

        // Build the "encrypted" flicker scene and "plaintext" resolve scene
        // for every character, then start everyone on the encrypted scene.
        for character in terminal.get_characters_mut() {
            let mut encrypted_scene = Scene::new("encrypted");
            for _ in 0..self.flicker_ticks {
                let idx = (next_rand(&mut rng_state) as usize) % self.symbol_set.len();
                let symbol = self.symbol_set[idx];
                let mut visual = CharacterVisual::new(symbol);
                visual.colors = Some(ColorPair::new(Some(self.ciphertext_color), None));
                visual.formatted_symbol = visual.format_symbol();
                encrypted_scene.add_frame(visual, self.frames_per_flicker_symbol);
            }
            character.animation.add_scene(encrypted_scene);

            let mut plaintext_scene = Scene::new("plaintext");
            let mut visual = CharacterVisual::new(character.input_symbol);
            visual.colors = Some(ColorPair::new(Some(self.plaintext_color), None));
            visual.formatted_symbol = visual.format_symbol();
            plaintext_scene.add_frame(visual, 1);
            character.animation.add_scene(plaintext_scene);

            character.animation.activate_scene("encrypted");
        }

        let ids: Vec<u32> = terminal.get_characters().iter().map(|c| c.id).collect();
        let char_count = ids.len();
        let mut frames = Vec::new();

        // Let the encrypted flicker play out for a bit before resolving
        // characters, mirroring the initial "ciphertext" impression.
        for _ in 0..self.flicker_ticks {
            terminal.step_animation();
            frames.push(terminal.render());
        }

        // Resolve characters to their real symbol, `typing_speed` per tick,
        // in input (id) order, until every character is plaintext.
        let mut resolved = 0usize;
        while resolved < char_count {
            for _ in 0..self.typing_speed {
                if resolved >= char_count {
                    break;
                }
                let id = ids[resolved];
                if let Some(character) = terminal.get_character_mut(id) {
                    character.animation.activate_scene("plaintext");
                }
                resolved += 1;
            }
            terminal.step_animation();
            frames.push(terminal.render());
        }

        frames
    }
}
