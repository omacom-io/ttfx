use crate::engine::canvas::CellStyle;
use crate::utils::geometry::Coord;

#[derive(Clone, Debug)]
pub struct EffectCharacter {
    pub id: u32,
    pub input_symbol: String,
    pub output_symbol: String,
    pub position: Coord,
    pub style: CellStyle,
    pub visible: bool,
}

impl EffectCharacter {
    pub fn new(id: u32, input_symbol: String, position: Coord) -> Self {
        Self {
            id,
            input_symbol: input_symbol.clone(),
            output_symbol: input_symbol,
            position,
            style: CellStyle::default(),
            visible: true,
        }
    }

    pub fn set_visibility(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}
