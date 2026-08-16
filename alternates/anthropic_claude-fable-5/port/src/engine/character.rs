//! EffectCharacter: one input character with animation + motion state.

use crate::engine::animation::{Animation, CharacterVisual};
use crate::engine::motion::Motion;
use crate::utils::geometry::Coord;

#[derive(Clone, Debug)]
pub struct EffectCharacter {
    pub character_id: usize,
    pub input_symbol: char,
    pub input_coord: Coord,
    pub is_visible: bool,
    pub animation: Animation,
    pub motion: Motion,
}

impl EffectCharacter {
    pub fn new(character_id: usize, input_symbol: char, input_coord: Coord) -> Self {
        EffectCharacter {
            character_id,
            input_symbol,
            input_coord,
            is_visible: false,
            animation: Animation::new(input_symbol),
            motion: Motion::new(input_coord),
        }
    }

    /// Advance animation and motion by one tick.
    pub fn tick(&mut self) {
        self.animation.step_animation();
        self.motion.move_char();
    }

    /// A character is active while it still has animation or motion pending.
    pub fn is_active(&self) -> bool {
        !(self.animation.active_scene_is_complete() && self.motion.movement_is_complete())
    }

    pub fn current_visual(&self) -> CharacterVisual {
        self.animation.current_visual.clone()
    }
}
