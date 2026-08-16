//! Overflow: rows of text tumble in from off-canvas above the viewport and
//! settle into their final positions, colored with a shifting gradient band
//! per source row (mirrors terminaltexteffects/effects/effect_overflow.py's
//! "overflow, scroll, and settle" shape within this skeleton's motion API).

use super::Effect;
use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::character::CharacterId;
use crate::engine::terminal::Terminal;
use crate::utils::easing::ease_out_cubic;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

pub struct Overflow;

impl Overflow {
    pub fn new() -> Self {
        Overflow
    }
}

struct CharAnim {
    id: CharacterId,
    target: Coord,
    start_row: f64,
    delay: usize,
    color: Color,
}

impl Effect for Overflow {
    fn name(&self) -> &str {
        "overflow"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let height = terminal.config.height as i32;

        // Palette mirroring the upstream effect's banded gradient look.
        let stops = [
            Color::Rgb(0xff, 0xff, 0xff),
            Color::Rgb(0x83, 0x3a, 0xb4),
            Color::Rgb(0xfd, 0x1d, 0x1d),
            Color::Rgb(0xfc, 0xb0, 0x45),
        ];
        let gradient = Gradient::new(&stops, 8);
        let gradient_len = gradient.len().max(1);

        let settle_duration: usize = 20;
        let max_delay = (height.max(1) - 1).max(0) as usize;
        let total_frames = max_delay + settle_duration + 1;

        // Build per-character animation plans: staggered entry from above the
        // canvas, easing down into the character's true input position.
        let mut anims: Vec<CharAnim> = Vec::with_capacity(terminal.get_characters().len());
        for character in terminal.get_characters() {
            let row = character.input_coord.row;
            let col = character.input_coord.column;
            let color_idx = (row.max(0) as usize) % gradient_len;
            let color = gradient.get(color_idx).unwrap_or(Color::Rgb(255, 255, 255));
            let start_row = -(height as f64) - (row as f64);
            let delay = row.max(0) as usize;
            anims.push(CharAnim {
                id: character.id,
                target: Coord::new(col, row),
                start_row,
                delay,
                color,
            });
        }

        // Assign each character its gradient-colored appearance up front.
        for anim in &anims {
            if let Some(ch) = terminal.get_character_mut(anim.id) {
                let mut visual = CharacterVisual::new(ch.input_symbol);
                visual.colors = Some(ColorPair::new(Some(anim.color), None));
                visual.formatted_symbol = visual.format_symbol();
                let mut scene = Scene::new("colored");
                scene.add_frame(visual, 1);
                ch.animation.add_scene(scene);
                ch.animation.activate_scene("colored");
            }
        }

        let mut frames_out = Vec::with_capacity(total_frames);
        for f in 0..total_frames {
            for anim in &anims {
                if let Some(ch) = terminal.get_character_mut(anim.id) {
                    let local_f = f.saturating_sub(anim.delay);
                    let t = (local_f as f64 / settle_duration as f64).clamp(0.0, 1.0);
                    let eased = ease_out_cubic(t);
                    let row = anim.start_row + (anim.target.row as f64 - anim.start_row) * eased;
                    let col = anim.target.column;
                    ch.motion.current_coord = Coord::new(col, row.round() as i32);
                    ch.motion.current_pos = (col as f64, row);
                }
            }
            frames_out.push(terminal.render());
        }

        frames_out
    }
}
