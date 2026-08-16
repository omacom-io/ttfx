//! Burn: the text ignites from the bottom of each column and burns upward,
//! each character cycling through fire symbols and colors before cooling
//! into its final gradient color.
//!
//! Port of terminaltexteffects/effects/effect_burn.py. The Python effect
//! sets every character visible in the starting color, orders characters by
//! popping the bottom-most character from a randomly chosen column, then per
//! frame releases a few characters whose "burn" scene (fire gradient over the
//! vertical build symbols) is followed — on scene completion — by a "burned"
//! scene fading from the ember color to the character's final gradient color.
//! The engine here has no event handler, so the scene-complete transition is
//! driven directly from the frame loop.

use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Symbols a character cycles through while burning (Python: vertical_build_order).
const VERTICAL_BUILD_ORDER: [char; 9] = ['\'', '.', '▖', '▙', '█', '▜', '▀', '▝', '.'];

/// Ticks each burn-scene frame is held.
const BURN_FRAME_DURATION: u32 = 2;
/// Ticks each burned-scene (cool-down) frame is held.
const BURNED_FRAME_DURATION: u32 = 5;
/// Interpolation steps between burned ember color and the final color.
const BURNED_GRADIENT_STEPS: usize = 8;

/// Minimal PRNG (LCG) standing in for Python's `random` module.
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9e37_79b9_7f4a_7c15);
        Rng(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 11
    }

    /// Uniform value in `0..n` (n >= 1).
    fn gen_range(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }

    /// Python-style inclusive randint.
    fn randint(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.gen_range(hi - lo + 1)
    }
}

pub struct Burn {
    starting_color: Color,
    burn_colors: Vec<Color>,
    final_gradient_stops: Vec<Color>,
    final_gradient_steps: usize,
}

impl Burn {
    pub fn new() -> Self {
        Burn {
            // Defaults from BurnConfig.
            starting_color: Color::from_hex("837373").expect("valid hex"),
            burn_colors: vec![
                Color::from_hex("ffffff").expect("valid hex"),
                Color::from_hex("fff75d").expect("valid hex"),
                Color::from_hex("fe650d").expect("valid hex"),
                Color::from_hex("8a003c").expect("valid hex"),
                Color::from_hex("510100").expect("valid hex"),
            ],
            final_gradient_stops: vec![
                Color::from_hex("00c3ff").expect("valid hex"),
                Color::from_hex("ffff1c").expect("valid hex"),
            ],
            final_gradient_steps: 12,
        }
    }
}

impl Default for Burn {
    fn default() -> Self {
        Burn::new()
    }
}

impl Effect for Burn {
    fn name(&self) -> &str {
        "burn"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let mut rng = Rng::new();

        if terminal.get_characters().is_empty() {
            return vec![terminal.get_formatted_output_string()];
        }

        // Text bounds for the vertical final-gradient mapping
        // (Python: final_gradient.build_coordinate_color_mapping over the text extents).
        let text_bottom = terminal
            .get_characters()
            .iter()
            .map(|c| c.input_coord.row)
            .min()
            .expect("non-empty");
        let text_top = terminal
            .get_characters()
            .iter()
            .map(|c| c.input_coord.row)
            .max()
            .expect("non-empty");

        let fire_gradient = Gradient::new(&self.burn_colors, 12);
        let final_gradient = Gradient::new(&self.final_gradient_stops, self.final_gradient_steps);
        let ember_color = *self.burn_colors.last().expect("burn colors non-empty");

        // Group characters by column; within a column, the bottom-most character
        // burns first (fire climbs upward).
        let mut groups: HashMap<i32, Vec<(i32, usize)>> = HashMap::new();
        for character in terminal.get_characters() {
            groups
                .entry(character.input_coord.column)
                .or_default()
                .push((character.input_coord.row, character.character_id));
        }
        let mut columns: Vec<i32> = groups.keys().copied().collect();
        columns.sort_unstable();
        for column in groups.values_mut() {
            // Descending row order so pop() removes the bottom-most character.
            column.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        }

        // Python: while groups remain, pop the next character from a random
        // non-empty column to build the pending order.
        let mut pending: VecDeque<usize> = VecDeque::new();
        loop {
            let nonempty: Vec<i32> = columns
                .iter()
                .copied()
                .filter(|c| !groups[c].is_empty())
                .collect();
            if nonempty.is_empty() {
                break;
            }
            let col = nonempty[rng.gen_range(nonempty.len())];
            if let Some((_, id)) = groups.get_mut(&col).and_then(|v| v.pop()) {
                pending.push_back(id);
            }
        }

        // Build scenes and set the starting appearance for every character.
        let fire_spectrum_len = fire_gradient.spectrum.len();
        for character in terminal.get_characters_mut() {
            let fraction = if text_top == text_bottom {
                0.0
            } else {
                (character.input_coord.row - text_bottom) as f64
                    / (text_top - text_bottom) as f64
            };
            let final_color = final_gradient
                .get_color_at_fraction(fraction)
                .unwrap_or(ember_color);

            // Python: set_appearance(input_symbol, starting_color) + visible in build().
            character.is_visible = true;
            character.animation.current_visual = CharacterVisual::new(
                character.input_symbol,
                false,
                ColorPair::fg(self.starting_color),
            );

            let input_symbol = character.input_symbol;

            // "burn" scene: fire gradient swept across the vertical build symbols.
            {
                let burn_scn = character.animation.new_scene("burn", false);
                for (i, color) in fire_gradient.spectrum.iter().enumerate() {
                    let t = if fire_spectrum_len > 1 {
                        i as f64 / (fire_spectrum_len - 1) as f64
                    } else {
                        0.0
                    };
                    let symbol_index =
                        (t * (VERTICAL_BUILD_ORDER.len() - 1) as f64).round() as usize;
                    burn_scn.add_frame(
                        VERTICAL_BUILD_ORDER[symbol_index],
                        BURN_FRAME_DURATION,
                        ColorPair::fg(*color),
                        false,
                    );
                }
            }

            // "burned" scene: cool from the ember color to the final gradient color.
            {
                let burned_gradient =
                    Gradient::new(&[ember_color, final_color], BURNED_GRADIENT_STEPS);
                let burned_scn = character.animation.new_scene("burned", false);
                for color in &burned_gradient.spectrum {
                    burned_scn.add_frame(
                        input_symbol,
                        BURNED_FRAME_DURATION,
                        ColorPair::fg(*color),
                        false,
                    );
                }
            }
        }

        // Frame loop. Python __next__: release a random 2..=4 pending characters
        // per frame, activating their burn scene; the event handler chains the
        // burned scene on SCENE_COMPLETE (done inline here).
        let mut frames = vec![terminal.get_formatted_output_string()];
        let mut safety = 0usize;
        loop {
            let animating = terminal
                .get_characters()
                .iter()
                .any(|c| !c.animation.active_scene_is_complete());
            if (pending.is_empty() && !animating) || safety > 40_000 {
                break;
            }
            safety += 1;

            let releases = rng.randint(2, 4);
            for _ in 0..releases {
                if let Some(id) = pending.pop_front() {
                    if let Some(character) = terminal
                        .get_characters_mut()
                        .iter_mut()
                        .find(|c| c.character_id == id)
                    {
                        character.animation.activate_scene("burn");
                    }
                }
            }

            terminal.tick();

            // SCENE_COMPLETE(burn) -> ACTIVATE_SCENE(burned)
            for character in terminal.get_characters_mut() {
                if character.animation.active_scene.as_deref() == Some("burn")
                    && character
                        .animation
                        .query_scene("burn")
                        .map(|s| s.complete)
                        .unwrap_or(false)
                {
                    character.animation.activate_scene("burned");
                }
            }

            frames.push(terminal.get_formatted_output_string());
        }

        frames
    }
}
