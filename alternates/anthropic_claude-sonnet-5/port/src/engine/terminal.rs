//! Terminal: canvas + character arena + rendering (mirrors the simulation
//! half of terminaltexteffects/engine/terminal.py, collapsed to a single
//! Terminal per plan.md §4.3).

use crate::engine::canvas::Canvas;
use crate::engine::character::{CharacterId, EffectCharacter};
use crate::utils::geometry::Coord;

#[derive(Debug, Clone, Copy)]
pub struct TerminalConfig {
    pub width: usize,
    pub height: usize,
}

/// Owns the character arena and canvas, and renders frames.
#[derive(Debug, Clone)]
pub struct Terminal {
    pub config: TerminalConfig,
    pub canvas: Canvas,
    characters: Vec<EffectCharacter>,
}

impl Terminal {
    /// Build a terminal from raw input text: one `EffectCharacter` per
    /// grid cell (including spaces), addressed by a monotonic id equal to
    /// its arena index, matching the plan's "reuse character_id as index"
    /// arena design (§4.1).
    pub fn new(input: &str) -> Self {
        let canvas = Canvas::from_text(input);
        let config = TerminalConfig { width: canvas.width, height: canvas.height };

        let mut characters = Vec::with_capacity(canvas.width * canvas.height);
        let mut next_id: CharacterId = 0;
        for row in 0..canvas.height {
            for column in 0..canvas.width {
                let symbol = canvas.get(column, row).unwrap_or(' ');
                let coord = Coord::new(column as i32, row as i32);
                characters.push(EffectCharacter::new(next_id, symbol, coord));
                next_id += 1;
            }
        }

        Terminal { config, canvas, characters }
    }

    pub fn get_characters(&self) -> &[EffectCharacter] {
        &self.characters
    }

    pub fn get_characters_mut(&mut self) -> &mut [EffectCharacter] {
        &mut self.characters
    }

    pub fn get_character(&self, id: CharacterId) -> Option<&EffectCharacter> {
        self.characters.get(id as usize)
    }

    pub fn get_character_mut(&mut self, id: CharacterId) -> Option<&mut EffectCharacter> {
        self.characters.get_mut(id as usize)
    }

    pub fn set_character_visibility(&mut self, id: CharacterId, is_visible: bool) {
        if let Some(character) = self.characters.get_mut(id as usize) {
            character.set_visibility(is_visible);
        }
    }

    /// Advance every character's animation and motion by one tick.
    pub fn step_animation(&mut self) {
        for character in &mut self.characters {
            character.animation.step_animation();
            character.motion.step();
        }
    }

    /// Render the current frame as a single string, one line per canvas row,
    /// overlaying each visible character's current formatted symbol at its
    /// motion-tracked coordinate onto a blank canvas backdrop.
    pub fn render(&self) -> String {
        let mut grid: Vec<Vec<String>> =
            vec![vec![" ".to_string(); self.config.width]; self.config.height];

        for character in &self.characters {
            if !character.visible {
                continue;
            }
            let coord = character.motion.current_coord;
            if coord.row < 0 || coord.column < 0 {
                continue;
            }
            let (row, column) = (coord.row as usize, coord.column as usize);
            if row < self.config.height && column < self.config.width {
                grid[row][column] = character.animation.current_visual().formatted_symbol.clone();
            }
        }

        grid.into_iter()
            .map(|row| row.join(""))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
