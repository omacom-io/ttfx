//! Pour effect: characters pour into place column by column, as if liquid
//! falling from above the canvas and settling into the input text, colored
//! along a watery gradient. Mirrors (simplified, given the engine's motion
//! primitives) terminaltexteffects/effects/effect_pour.py's group-based
//! pour-and-settle behavior.

use std::collections::{BTreeMap, HashMap};

use super::Effect;
use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::character::CharacterId;
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

pub struct Pour;

impl Pour {
    pub fn new() -> Self {
        Pour
    }
}

impl Effect for Pour {
    fn name(&self) -> &str {
        "pour"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let height = terminal.config.height;

        // Watery pour gradient: deep blue "source" through to pale
        // cyan/white "settled" tone, mirroring the upstream pour effect's
        // default color ramp.
        let stops = [
            Color::Rgb(0, 60, 180),
            Color::Rgb(0, 160, 220),
            Color::Rgb(210, 240, 255),
        ];
        let gradient = Gradient::new(&stops, 12);
        let gradient_len = gradient.len().max(1);

        // Snapshot (id, column, row) before taking any mutable borrows.
        let entries: Vec<(CharacterId, i32, i32)> = terminal
            .get_characters()
            .iter()
            .map(|c| (c.id, c.input_coord.column, c.input_coord.row))
            .collect();

        // Group characters into columns; each column pours in as a unit,
        // columns left-to-right, mirroring the upstream "down" pour
        // direction's group-by-column stagger.
        let mut columns: BTreeMap<i32, Vec<CharacterId>> = BTreeMap::new();
        for (id, column, _) in &entries {
            columns.entry(*column).or_default().push(*id);
        }

        let stagger_per_column: usize = 2;
        let fall_duration: usize = 10;

        let mut start_frame_map: HashMap<CharacterId, usize> = HashMap::new();
        for (column_index, (_, ids)) in columns.iter().enumerate() {
            for id in ids {
                start_frame_map.insert(*id, column_index * stagger_per_column);
            }
        }

        let mut target_row_map: HashMap<CharacterId, i32> = HashMap::new();
        let mut column_map: HashMap<CharacterId, i32> = HashMap::new();
        for (id, column, row) in &entries {
            target_row_map.insert(*id, *row);
            column_map.insert(*id, *column);
        }

        // Pre-bake each character's gradient-colored appearance (keyed off
        // its landing row) and hide it until its pour column starts.
        for character in terminal.get_characters_mut() {
            let target_row = *target_row_map.get(&character.id).unwrap_or(&0);
            let gradient_index = (target_row.max(0) as usize) % gradient_len;
            let color = gradient.get(gradient_index).unwrap_or(Color::Rgb(255, 255, 255));

            let mut visual = CharacterVisual::new(character.input_symbol);
            visual.colors = Some(ColorPair::new(Some(color), None));
            visual.formatted_symbol = visual.format_symbol();

            let mut scene = Scene::new("pour_color");
            scene.add_frame(visual, u32::MAX / 2);
            character.animation.add_scene(scene);
            character.animation.activate_scene("pour_color");

            character.set_visibility(false);
        }

        let ordered_ids: Vec<CharacterId> = entries.iter().map(|(id, _, _)| *id).collect();

        let max_column_index = columns.len().saturating_sub(1);
        let total_frames = (max_column_index * stagger_per_column + fall_duration + height + 2).max(1);

        let mut frames = Vec::with_capacity(total_frames);
        for frame_idx in 0..total_frames {
            for id in &ordered_ids {
                let start = *start_frame_map.get(id).unwrap_or(&0);
                if frame_idx < start {
                    continue;
                }

                let column = *column_map.get(id).unwrap_or(&0);
                let target_row = *target_row_map.get(id).unwrap_or(&0);

                if let Some(character) = terminal.get_character_mut(*id) {
                    character.set_visibility(true);
                    let elapsed = (frame_idx - start).min(fall_duration);
                    let t = elapsed as f64 / fall_duration as f64;
                    let eased_t = easing::ease_out_cubic(t);
                    let start_row = -1.0_f64;
                    let row_f = start_row + eased_t * (target_row as f64 - start_row);
                    character.motion.current_pos = (column as f64, row_f);
                    character.motion.current_coord = Coord::new(column, row_f.round() as i32);
                }
            }
            terminal.step_animation();
            frames.push(terminal.render());
        }

        frames
    }
}
