use std::collections::HashMap;

use super::Effect;
use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::character::CharacterId;
use crate::engine::motion::Path;
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::{self, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Rings: characters converge onto their home position from concentric
/// rings around the canvas center, staggered ring-by-ring, mirroring the
/// overall shape of `terminaltexteffects/effects/effect_rings.py` within
/// the constraints of this engine skeleton (no event handler / bezier
/// waypoints available here, so paths are simple two-waypoint eased moves).
pub struct Rings;

impl Rings {
    pub fn new() -> Self {
        Rings
    }
}

impl Effect for Rings {
    fn name(&self) -> &str {
        "rings"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);

        let center_col = (terminal.canvas.width as i32 - 1) / 2;
        let center_row = (terminal.canvas.height as i32 - 1) / 2;
        let center = Coord::new(center_col, center_row);

        let ring_gap: i32 = 3;
        let speed: f64 = 0.8;
        let stagger: usize = 3;

        // Group characters by ring index based on distance from center.
        let char_ids: Vec<CharacterId> = terminal.get_characters().iter().map(|c| c.id).collect();
        let mut ring_groups: HashMap<i32, Vec<CharacterId>> = HashMap::new();
        for id in &char_ids {
            let coord = terminal.get_character(*id).unwrap().input_coord;
            let dist = geometry::distance(center, coord);
            let ring_index = (dist / ring_gap as f64).floor() as i32;
            ring_groups.entry(ring_index).or_default().push(*id);
        }

        let mut ring_indices: Vec<i32> = ring_groups.keys().copied().collect();
        ring_indices.sort_unstable();

        // Color gradient across rings.
        let stops = [Color::Rgb(255, 60, 60), Color::Rgb(60, 200, 255), Color::Rgb(255, 255, 255)];
        let gradient = Gradient::new(&stops, 10);
        let gradient_len = gradient.len().max(1);

        let mut max_ticks: usize = 0;

        for (ring_pos, &ring_idx) in ring_indices.iter().enumerate() {
            let ids = ring_groups.get(&ring_idx).cloned().unwrap_or_default();
            let radius = ((ring_idx * ring_gap).max(1)) as f64;
            let count = ids.len().max(1);

            let color_idx = (ring_pos * 7) % gradient_len;
            let color = gradient.get(color_idx).unwrap_or(Color::Rgb(255, 255, 255));

            for (i, id) in ids.iter().copied().enumerate() {
                let angle = 2.0 * std::f64::consts::PI * (i as f64) / (count as f64);
                let start_col = center.column as f64 + radius * angle.cos();
                let start_row = center.row as f64 + radius * angle.sin();
                let start_coord = Coord::new(start_col.round() as i32, start_row.round() as i32);

                let character = terminal.get_character_mut(id).unwrap();

                character.motion.current_pos = (start_coord.column as f64, start_coord.row as f64);
                character.motion.current_coord = start_coord;

                let mut path = Path::new("home", speed);
                path.ease = Some(easing::ease_out_quad);
                path.add_waypoint(start_coord);
                path.add_waypoint(character.input_coord);
                let path_total = path.total_distance();
                character.motion.add_path(path);

                let mut scene = Scene::new("ring");
                let mut visual = CharacterVisual::new(character.input_symbol);
                visual.colors = Some(ColorPair::new(Some(color), None));
                visual.formatted_symbol = visual.format_symbol();
                scene.add_frame(visual, 1);
                character.animation.add_scene(scene);

                character.set_visibility(false);

                let ticks_needed = (path_total / speed).ceil() as usize;
                let total_needed = ring_pos * stagger + ticks_needed;
                if total_needed > max_ticks {
                    max_ticks = total_needed;
                }
            }
        }

        let total_frames = max_ticks + stagger + 5;
        let mut frames = Vec::with_capacity(total_frames);

        for tick in 0..total_frames {
            let ring_to_activate = tick / stagger.max(1);
            if ring_to_activate < ring_indices.len() {
                let ring_idx = ring_indices[ring_to_activate];
                if let Some(ids) = ring_groups.get(&ring_idx) {
                    for &id in ids {
                        if let Some(character) = terminal.get_character_mut(id) {
                            if !character.visible {
                                character.set_visibility(true);
                                character.animation.activate_scene("ring");
                                character.motion.activate_path("home");
                            }
                        }
                    }
                }
            }

            terminal.step_animation();
            frames.push(terminal.render());
        }

        frames
    }
}
