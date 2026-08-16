use crate::engine::animation::Animation;
use crate::engine::motion::Motion;
use crate::utils::geometry::Coord;
use crate::utils::graphics::Style;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharacterId(pub u32);

#[derive(Debug, Clone)]
pub struct EffectCharacter {
    pub id: CharacterId,
    pub input_symbol: char,
    pub symbol: char,
    pub position: Coord,
    pub style: Style,
    pub visible: bool,
    pub animation: Animation,
    pub motion: Motion,
}

impl EffectCharacter {
    pub fn new(id: CharacterId, symbol: char, position: Coord) -> Self {
        Self {
            id,
            input_symbol: symbol,
            symbol,
            position,
            style: Style::default(),
            visible: true,
            animation: Animation::new(symbol, Style::default()),
            motion: Motion::default(),
        }
    }

    pub fn set_position(&mut self, position: Coord) {
        self.position = position;
    }

    pub fn set_style(&mut self, style: Style) {
        self.style = style.clone();
        self.animation.set_appearance(self.symbol, style);
    }

    pub fn set_appearance(&mut self, symbol: char, style: Style) {
        self.symbol = symbol;
        self.style = style.clone();
        self.animation.set_appearance(symbol, style);
    }

    pub fn step(&mut self) {
        if let Some(position) = self.motion.step() {
            self.position = position;
        }

        if let Some(visual) = self.animation.step() {
            self.symbol = visual.symbol;
            self.style = visual.style;
        }
    }

    pub fn reset(&mut self) {
        self.symbol = self.input_symbol;
        self.visible = true;
        self.animation.deactivate();
        self.motion.deactivate();
    }
}
