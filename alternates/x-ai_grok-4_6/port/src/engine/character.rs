use crate::engine::animation::Animation;
use crate::engine::motion::Motion;
use crate::utils::geometry::Coord;
use crate::utils::graphics::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CharacterId(pub u32);

#[derive(Clone, Debug, Default)]
pub struct Neighbors {
    pub above: Option<CharacterId>,
    pub below: Option<CharacterId>,
    pub left: Option<CharacterId>,
    pub right: Option<CharacterId>,
}

#[derive(Clone, Debug)]
pub struct EffectCharacter {
    pub id: CharacterId,
    pub character_id: u32,
    pub input_symbol: String,
    pub input_coord: Coord,
    pub input_fg: Option<Color>,
    pub input_bg: Option<Color>,
    pub is_visible: bool,
    pub animation: Animation,
    pub motion: Motion,
    pub neighbors: Neighbors,
}

impl EffectCharacter {
    pub fn new(id: CharacterId, symbol: impl Into<String>, coord: Coord) -> Self {
        let input_symbol = symbol.into();
        Self {
            id,
            character_id: id.0,
            input_symbol: input_symbol.clone(),
            input_coord: coord,
            input_fg: None,
            input_bg: None,
            is_visible: false,
            animation: Animation::new(input_symbol),
            motion: Motion::new(coord),
            neighbors: Neighbors::default(),
        }
    }

    pub fn current_coord(&self) -> Coord {
        self.motion.current_coord
    }

    pub fn is_active(&self) -> bool {
        self.animation.is_active() || self.motion.is_active()
    }

    pub fn tick(&mut self) {
        self.animation.step_animation();
        self.motion.move_character();
    }
}
