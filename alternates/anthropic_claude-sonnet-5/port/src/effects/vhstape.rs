//! VHS tape glitch effect (mirrors terminaltexteffects/effects/effect_vhstape.py).
//!
//! The upstream effect randomly triggers short horizontal "glitch waves" on
//! scan-line groups of the input, swapping in chromatic-aberration-shifted
//! colors while the wave passes and settling back to a stable color once it
//! clears. The engine skeleton available here has no RNG module, so the
//! glitch schedule is derived deterministically from `(frame, row)` instead
//! of `random.random()`/`random.randint()` — the visual shape (rows glitch
//! in short bursts, shift left/right briefly, then settle) is preserved.

use super::Effect;
use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::terminal::Terminal;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair};

const TOTAL_FRAMES: i32 = 45;
const GLITCH_PERIOD: i32 = 7;
const GLITCH_SPAN: i32 = 3;

pub struct Vhstape {
    name: String,
}

impl Vhstape {
    pub fn new() -> Self {
        Vhstape { name: "vhstape".to_string() }
    }
}

impl Effect for Vhstape {
    fn name(&self) -> &str {
        &self.name
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let width = terminal.config.width as i32;

        // Build the two scenes each character alternates between: a stable
        // base appearance, and a chromatic-aberration-shifted glitch
        // appearance, mirroring `base_scn`/`glitch_scn_forward` upstream.
        for character in terminal.get_characters_mut() {
            let input_symbol = character.input_symbol;

            let mut base_visual = CharacterVisual::new(input_symbol);
            base_visual.colors = Some(ColorPair::new(Some(Color::Rgb(210, 210, 210)), None));
            base_visual.formatted_symbol = base_visual.format_symbol();
            let mut base_scn = Scene::new("base");
            base_scn.add_frame(base_visual, 1);
            character.animation.add_scene(base_scn);

            let mut glitch_visual = CharacterVisual::new(input_symbol);
            glitch_visual.colors =
                Some(ColorPair::new(Some(Color::Rgb(255, 0, 255)), Some(Color::Rgb(0, 255, 255))));
            glitch_visual.formatted_symbol = glitch_visual.format_symbol();
            let mut glitch_scn = Scene::new("glitch");
            glitch_scn.add_frame(glitch_visual, 1);
            character.animation.add_scene(glitch_scn);

            character.animation.activate_scene("base");
        }

        let mut frames = Vec::new();

        for frame_num in 0..TOTAL_FRAMES {
            for character in terminal.get_characters_mut() {
                let input_coord = character.input_coord;
                let row = input_coord.row;
                let col = input_coord.column;

                // Deterministic stand-in for the upstream random glitch
                // trigger: each row enters a short glitch burst once per
                // `GLITCH_PERIOD` frames, offset by its own row so scan
                // lines don't all glitch in lockstep.
                let phase = (frame_num + row).rem_euclid(GLITCH_PERIOD);
                let glitching = phase < GLITCH_SPAN;

                if glitching {
                    character.animation.activate_scene("glitch");
                    // Tracking-error style horizontal jitter: shift right on
                    // the first tick of the burst, left on the last, hold on
                    // the middle tick — a stand-in for the wave's forward/
                    // reverse glitch scenes sweeping the character sideways.
                    let shift = match phase {
                        0 => 2,
                        _ if phase == GLITCH_SPAN - 1 => -2,
                        _ => 0,
                    };
                    let new_col = (col + shift).clamp(0, width.saturating_sub(1).max(0));
                    character.motion.current_coord = Coord::new(new_col, row);
                } else {
                    character.animation.activate_scene("base");
                    character.motion.current_coord = input_coord;
                }
            }

            terminal.step_animation();
            frames.push(terminal.render());
        }

        frames
    }
}
