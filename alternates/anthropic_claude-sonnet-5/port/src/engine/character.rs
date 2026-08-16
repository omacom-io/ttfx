//! EffectCharacter: per-character storage (mirrors
//! terminaltexteffects/engine/base_character.py, minus event handler
//! dispatch which lands with the effects/events work).

use crate::engine::animation::Animation;
use crate::engine::motion::Motion;
use crate::utils::geometry::Coord;

pub type CharacterId = u32;

#[derive(Debug, Clone)]
pub struct EffectCharacter {
    pub id: CharacterId,
    pub input_symbol: char,
    pub input_coord: Coord,
    pub visible: bool,
    pub animation: Animation,
    pub motion: Motion,
}

impl EffectCharacter {
    pub fn new(id: CharacterId, input_symbol: char, input_coord: Coord) -> Self {
        EffectCharacter {
            id,
            input_symbol,
            input_coord,
            visible: true,
            animation: Animation::new(input_symbol),
            motion: Motion::new(input_coord),
        }
    }

    pub fn set_visibility(&mut self, is_visible: bool) {
        self.visible = is_visible;
    }
}
