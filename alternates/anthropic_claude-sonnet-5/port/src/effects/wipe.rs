//! Wipe: reveals the input text one column at a time (left to right),
//! sweeping each newly revealed column through a short color gradient
//! before it settles into its final color. Mirrors the shape of upstream's
//! `effect_wipe.py` (grouped reveal-by-direction + gradient-then-settle
//! animation), simplified to the single `column_left_to_right` direction and
//! a fixed gradient/timing configuration, since this port's `Effect` trait
//! takes no per-run CLI config.

use std::collections::BTreeMap;

use super::Effect;
use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Number of animation ticks each gradient frame is held for.
const FRAME_DURATION: u32 = 4;
/// Number of ticks between activating successive columns.
const GROUP_DELAY: u32 = 3;

pub struct Wipe {
    name: String,
}

impl Wipe {
    pub fn new() -> Self {
        Wipe { name: "wipe".to_string() }
    }
}

impl Effect for Wipe {
    fn name(&self) -> &str {
        &self.name
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);

        // Start with every character hidden; each column is revealed in turn.
        let all_ids: Vec<u32> = terminal.get_characters().iter().map(|c| c.id).collect();
        for id in &all_ids {
            terminal.set_character_visibility(*id, false);
        }

        // Group character ids by their input column, matching the upstream
        // "column_left_to_right" wipe direction grouping.
        let mut groups: BTreeMap<i32, Vec<u32>> = BTreeMap::new();
        for character in terminal.get_characters() {
            groups.entry(character.input_coord.column).or_default().push(character.id);
        }
        let columns: Vec<i32> = groups.keys().copied().collect();

        // Gradient the wipe sweeps through before settling on the final color.
        let gradient_stops = [Color::Rgb(0x83, 0x3a, 0xb4), Color::Rgb(0xfd, 0x1d, 0x1d), Color::Rgb(0xfc, 0xb0, 0x45)];
        let gradient = Gradient::new(&gradient_stops, 5);
        let final_color = Color::Rgb(0xff, 0xff, 0xff);

        // Build the "wipe" scene for every character up front: a run through
        // the gradient spectrum, then a hold on the final settled color.
        for character in terminal.get_characters_mut() {
            let mut scene = Scene::new("wipe");
            for i in 0..gradient.len() {
                let color = gradient.get(i).unwrap_or(final_color);
                let mut visual = CharacterVisual::new(character.input_symbol);
                visual.colors = Some(ColorPair::new(Some(color), None));
                visual.formatted_symbol = visual.format_symbol();
                scene.add_frame(visual, FRAME_DURATION);
            }
            let mut final_visual = CharacterVisual::new(character.input_symbol);
            final_visual.colors = Some(ColorPair::new(Some(final_color), None));
            final_visual.formatted_symbol = final_visual.format_symbol();
            scene.add_frame(final_visual, 1);
            character.animation.add_scene(scene);
        }

        let mut frames_out = Vec::new();
        let mut next_group_idx = 0usize;
        let mut ticks_until_next_group = 0u32;

        loop {
            if next_group_idx < columns.len() && ticks_until_next_group == 0 {
                let col = columns[next_group_idx];
                for id in &groups[&col] {
                    terminal.set_character_visibility(*id, true);
                    if let Some(character) = terminal.get_character_mut(*id) {
                        character.animation.activate_scene("wipe");
                    }
                }
                next_group_idx += 1;
                ticks_until_next_group = GROUP_DELAY;
            } else if ticks_until_next_group > 0 {
                ticks_until_next_group -= 1;
            }

            terminal.step_animation();
            frames_out.push(terminal.render());

            if next_group_idx >= columns.len() {
                break;
            }
        }

        // Let the final activated column's gradient sweep finish playing out.
        let total_scene_ticks = FRAME_DURATION * gradient.len() as u32 + 1;
        for _ in 0..total_scene_ticks {
            terminal.step_animation();
            frames_out.push(terminal.render());
        }

        frames_out
    }
}
