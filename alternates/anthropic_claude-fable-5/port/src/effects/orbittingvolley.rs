//! Orbittingvolley: four launchers orbit the canvas perimeter, firing
//! characters toward their input coordinates in successive volleys.
//! Port of terminaltexteffects/effects/effect_orbittingvolley.py.

use super::Effect;
use crate::engine::character::EffectCharacter;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

const LAUNCHER_SYMBOL: char = '█';
const LAUNCHER_MOVEMENT_SPEED: f64 = 0.5;
const CHARACTER_MOVEMENT_SPEED: f64 = 1.0;
const VOLLEY_SIZE: f64 = 0.03;
const LAUNCH_DELAY: u32 = 25;
const MAX_TICKS: u32 = 20_000;

pub struct Orbittingvolley;

impl Orbittingvolley {
    pub fn new() -> Self {
        Orbittingvolley
    }
}

impl Default for Orbittingvolley {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Orbittingvolley {
    fn name(&self) -> &str {
        "orbittingvolley"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;
        let (left, right, bottom, top) = (1, width, 1, height);

        // Final gradient (upstream defaults: FFA15C -> 44D492).
        let stops = [
            Color::from_hex("FFA15C").expect("valid hex"),
            Color::from_hex("44D492").expect("valid hex"),
        ];
        let gradient = Gradient::new(&stops, 12);

        let char_count = terminal.characters.len();

        // Prepare every input character: hidden, colored by the final gradient
        // (vertical direction), waiting in a launcher magazine.
        for character in terminal.get_characters_mut() {
            let fraction = if height > 1 {
                (character.input_coord.row - 1) as f64 / (height - 1) as f64
            } else {
                0.0
            };
            let color = gradient.get_color_at_fraction(fraction);
            let scene = character.animation.new_scene("final", false);
            scene.add_frame(
                character.input_symbol,
                1,
                ColorPair::new(color, None),
                false,
            );
            character.animation.activate_scene("final");
            character.is_visible = false;
        }

        // Distribute characters round-robin among the four launcher magazines.
        let mut magazines: Vec<Vec<usize>> = vec![Vec::new(); 4];
        for (index, character) in terminal.get_characters().iter().enumerate() {
            magazines[index % 4].push(character.character_id);
        }

        // Build the four launchers, one per canvas corner, each with a
        // perimeter path that starts at its own corner and circles back.
        let corners = [
            Coord::new(left, top),
            Coord::new(right, top),
            Coord::new(right, bottom),
            Coord::new(left, bottom),
        ];
        let mut launcher_ids: Vec<usize> = Vec::with_capacity(4);
        for (i, &corner) in corners.iter().enumerate() {
            let id = char_count + i;
            let mut launcher = EffectCharacter::new(id, LAUNCHER_SYMBOL, corner);
            launcher.is_visible = true;

            // Looping scene cycling through the gradient spectrum.
            let scene = launcher.animation.new_scene("orbit", true);
            if gradient.spectrum.is_empty() {
                scene.add_frame(LAUNCHER_SYMBOL, 1, ColorPair::default(), false);
            } else {
                for color in &gradient.spectrum {
                    scene.add_frame(LAUNCHER_SYMBOL, 3, ColorPair::fg(*color), false);
                }
            }
            launcher.animation.activate_scene("orbit");

            // Perimeter path: this corner, around all corners, back to start.
            let path = launcher
                .motion
                .new_path("perimeter", LAUNCHER_MOVEMENT_SPEED, None);
            for j in 0..=4usize {
                path.add_waypoint(corners[(i + j) % 4]);
            }
            launcher.motion.activate_path("perimeter");

            launcher_ids.push(id);
            terminal.characters.push(launcher);
        }

        let chars_per_launch = ((VOLLEY_SIZE * char_count as f64) as usize).max(1);

        let mut frames: Vec<String> = Vec::new();
        let mut delay: u32 = 0;
        let mut next_launcher: usize = 0;
        let mut ticks: u32 = 0;

        loop {
            ticks += 1;
            if ticks > MAX_TICKS {
                break;
            }

            let all_magazines_empty = magazines.iter().all(|m| m.is_empty());

            // Launch a volley when the delay has elapsed.
            if !all_magazines_empty {
                if delay == 0 {
                    let mut to_launch = chars_per_launch;
                    let mut misses = 0usize;
                    while to_launch > 0 && misses < 4 {
                        let li = next_launcher % 4;
                        next_launcher = next_launcher.wrapping_add(1);
                        if magazines[li].is_empty() {
                            misses += 1;
                            continue;
                        }
                        misses = 0;
                        let char_id = magazines[li].remove(0);
                        let launcher_coord = terminal
                            .characters
                            .iter()
                            .find(|c| c.character_id == launcher_ids[li])
                            .map(|c| c.motion.current_coord)
                            .unwrap_or(corners[li]);
                        if let Some(character) = terminal
                            .characters
                            .iter_mut()
                            .find(|c| c.character_id == char_id)
                        {
                            let target = character.input_coord;
                            character.motion.current_coord = launcher_coord;
                            let path = character.motion.new_path(
                                "input_path",
                                CHARACTER_MOVEMENT_SPEED,
                                Some(easing::out_sine),
                            );
                            path.add_waypoint(launcher_coord);
                            path.add_waypoint(target);
                            character.motion.activate_path("input_path");
                            character.is_visible = true;
                        }
                        to_launch -= 1;
                    }
                    delay = LAUNCH_DELAY;
                } else {
                    delay -= 1;
                }
            }

            // Keep launchers orbiting: reactivate the perimeter path on completion.
            for &id in &launcher_ids {
                if let Some(launcher) = terminal
                    .characters
                    .iter_mut()
                    .find(|c| c.character_id == id)
                {
                    if launcher.motion.movement_is_complete() {
                        launcher.motion.activate_path("perimeter");
                    }
                }
            }

            let _ = terminal.tick();
            frames.push(terminal.get_formatted_output_string());

            // Finished when every character has been launched and settled.
            if all_magazines_empty {
                let all_settled = terminal
                    .characters
                    .iter()
                    .filter(|c| c.character_id < char_count)
                    .all(|c| c.motion.movement_is_complete());
                if all_settled {
                    break;
                }
            }
        }

        // Final frame: launchers gone, text resolved in place.
        for &id in &launcher_ids {
            terminal.set_character_visibility(id, false);
        }
        frames.push(terminal.get_formatted_output_string());

        frames
    }
}
