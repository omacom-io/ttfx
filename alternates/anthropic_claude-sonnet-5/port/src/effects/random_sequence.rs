//! RandomSequence effect: characters are revealed in a randomized order,
//! each taking on a color drawn from a gradient spanning the canvas width.
//! Mirrors the shape of `terminaltexteffects/effects/effect_random_sequence.py`
//! (staggered per-character reveal order + static gradient-derived coloring),
//! adapted to the primitives actually exposed by this port's engine.

use super::Effect;
use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Minimal, self-contained xorshift64* PRNG. No external `rand` dependency is
/// assumed to be available to this crate, so we roll a tiny deterministic
/// generator sufficient for shuffling reveal order.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        // Avoid an all-zero state, which would make xorshift64* degenerate.
        let seeded = seed ^ 0x9E3779B97F4A7C15;
        Rng { state: if seeded == 0 { 0xD1B54A32D192ED03 } else { seeded } }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn gen_range_usize(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() as usize) % n
        }
    }

    /// Fisher-Yates shuffle, mirroring the effect of Python's `random.shuffle`.
    fn shuffle<T>(&mut self, v: &mut [T]) {
        let len = v.len();
        if len < 2 {
            return;
        }
        for i in (1..len).rev() {
            let j = self.gen_range_usize(i + 1);
            v.swap(i, j);
        }
    }
}

pub struct RandomSequence {
    name: String,
}

impl RandomSequence {
    pub fn new() -> Self {
        RandomSequence { name: "random_sequence".to_string() }
    }
}

impl Effect for RandomSequence {
    fn name(&self) -> &str {
        &self.name
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let total_chars = terminal.get_characters().len();

        // A rainbow-ish gradient spanning the canvas width; each character's
        // final color is chosen by its input column, matching the way
        // upstream effects derive per-character color from a precomputed
        // Gradient spectrum.
        let width = terminal.config.width.max(1);
        let stops = vec![
            Color::Rgb(255, 64, 64),
            Color::Rgb(255, 255, 64),
            Color::Rgb(64, 255, 128),
            Color::Rgb(64, 191, 255),
            Color::Rgb(191, 64, 255),
        ];
        let gradient = Gradient::new(&stops, width);

        // Deterministic pseudo-random reveal order (mirrors upstream's
        // `random.shuffle`-based staggering of the character reveal
        // sequence).
        let mut order: Vec<usize> = (0..total_chars).collect();
        let mut rng = Rng::new(0x5EED_1234u64 ^ (input.len() as u64).wrapping_add(total_chars as u64));
        rng.shuffle(&mut order);

        // Hide every character and give each one a static, gradient-derived
        // colored appearance scene up front. The scene is a single
        // non-looping frame, so once activated it holds its color for the
        // remainder of the run (mirrors a "final" colored appearance scene).
        for character in terminal.get_characters_mut().iter_mut() {
            character.set_visibility(false);

            let col = character.input_coord.column.max(0) as usize;
            let idx = col.min(gradient.len().saturating_sub(1));
            let color = gradient.get(idx).unwrap_or(Color::Rgb(255, 255, 255));

            let mut visual = CharacterVisual::new(character.input_symbol);
            visual.colors = Some(ColorPair::new(Some(color), None));
            visual.formatted_symbol = visual.format_symbol();

            let mut scene = Scene::new(format!("revealed_{}", character.id));
            scene.add_frame(visual, 1);
            let scene_id = scene.id.clone();
            character.animation.add_scene(scene);
            character.animation.activate_scene(&scene_id);
        }

        // Reveal characters a handful at a time, in the shuffled order,
        // rendering one frame per batch so the sequence appears staggered
        // rather than instantaneous.
        let batch_size = (total_chars / 40).max(1);
        let mut frames = Vec::new();
        let mut revealed = 0usize;
        while revealed < total_chars {
            let end = (revealed + batch_size).min(total_chars);
            for &idx in &order[revealed..end] {
                if let Some(character) = terminal.get_character_mut(idx as u32) {
                    character.set_visibility(true);
                }
            }
            revealed = end;
            terminal.step_animation();
            frames.push(terminal.render());
        }

        // A few settle frames once everything is revealed.
        for _ in 0..5 {
            terminal.step_animation();
            frames.push(terminal.render());
        }

        frames
    }
}
