//! EffectCharacter, ported from engine/base_character.py, stored in an arena.
//!
//! `CharId` is the arena slot index. `character_id` is the Python-compatible
//! monotonically allocated id — these are NOT the same thing: the Python parser
//! allocates ids for characters that are later overwritten by cursor movement,
//! popped as trailing whitespace, or cropped by the canvas, so surviving
//! characters have id gaps. All canonical orderings sort by `character_id`.

use crate::engine::animation::Animation;
use crate::engine::motion::Motion;
use crate::utils::geometry::Coord;

/// Arena slot index (dense). Never used for ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CharId(pub u32);

/// Cardinal neighbor slots, upstream's dict keys north/east/south/west.
#[derive(Debug, Clone, Copy, Default)]
pub struct Neighbors {
    pub north: Option<CharId>,
    pub east: Option<CharId>,
    pub south: Option<CharId>,
    pub west: Option<CharId>,
}

#[derive(Debug, Clone)]
pub struct EffectCharacter {
    /// Python-compatible allocation id; canonical ordering key.
    pub character_id: u32,
    pub input_symbol: String,
    pub input_coord: Coord,
    /// Raw input SGR sequences captured at parse time (fg, bg).
    pub input_ansi_fg_sequence: Option<String>,
    pub input_ansi_bg_sequence: Option<String>,
    pub is_visible: bool,
    pub animation: Animation,
    pub motion: Motion,
    pub layer: i64,
    pub is_fill_character: bool,
    pub uses_input_preexisting_colors: bool,
    /// Spanning-tree links (character ids, insertion-ordered — upstream uses a
    /// set, but iteration order is behavior; see plan.md §4.3).
    pub links: Vec<CharId>,
    pub neighbors: Neighbors,
}

impl EffectCharacter {
    pub fn new(character_id: u32, symbol: &str, input_column: i64, input_row: i64) -> Self {
        let input_coord = Coord::new(input_column, input_row);
        EffectCharacter {
            character_id,
            input_symbol: symbol.to_string(),
            input_coord,
            input_ansi_fg_sequence: None,
            input_ansi_bg_sequence: None,
            is_visible: false,
            animation: Animation::new(symbol),
            motion: Motion::new(input_coord),
            layer: 0,
            is_fill_character: false,
            uses_input_preexisting_colors: false,
            links: Vec::new(),
            neighbors: Neighbors::default(),
        }
    }
}
