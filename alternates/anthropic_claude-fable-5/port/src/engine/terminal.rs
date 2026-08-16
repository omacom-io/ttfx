//! Terminal: owns the canvas and the character arena; produces frame strings.

use crate::engine::canvas::Canvas;
use crate::engine::character::EffectCharacter;
use crate::utils::geometry::Coord;

#[derive(Clone, Debug)]
pub struct TerminalConfig {
    /// 0 means "size to input".
    pub canvas_width: usize,
    /// 0 means "size to input".
    pub canvas_height: usize,
    pub frame_rate: u64,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        TerminalConfig {
            canvas_width: 0,
            canvas_height: 0,
            frame_rate: 60,
        }
    }
}

pub struct Terminal {
    pub config: TerminalConfig,
    pub canvas: Canvas,
    pub characters: Vec<EffectCharacter>,
}

impl Terminal {
    /// Parse input text into a character arena placed at the top of the canvas.
    pub fn new(input: &str, config: TerminalConfig) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        let width = if config.canvas_width > 0 {
            config.canvas_width
        } else {
            lines
                .iter()
                .map(|l| l.chars().count())
                .max()
                .unwrap_or(1)
                .max(1)
        };
        let height = if config.canvas_height > 0 {
            config.canvas_height
        } else {
            lines.len().max(1)
        };

        let canvas = Canvas::new(width, height);
        let mut characters = Vec::new();
        let mut next_id = 0usize;

        for (line_idx, line) in lines.iter().enumerate() {
            let row = height as i32 - line_idx as i32;
            if row < 1 {
                break;
            }
            for (col_idx, symbol) in line.chars().take(width).enumerate() {
                if symbol == ' ' {
                    continue;
                }
                let coord = Coord::new(col_idx as i32 + 1, row);
                characters.push(EffectCharacter::new(next_id, symbol, coord));
                next_id += 1;
            }
        }

        Terminal {
            config,
            canvas,
            characters,
        }
    }

    pub fn get_characters(&self) -> &[EffectCharacter] {
        &self.characters
    }

    pub fn get_characters_mut(&mut self) -> &mut [EffectCharacter] {
        &mut self.characters
    }

    pub fn set_character_visibility(&mut self, character_id: usize, is_visible: bool) {
        if let Some(character) = self
            .characters
            .iter_mut()
            .find(|c| c.character_id == character_id)
        {
            character.is_visible = is_visible;
        }
    }

    /// Step every character one tick; returns the number still active.
    pub fn tick(&mut self) -> usize {
        let mut active = 0;
        for character in &mut self.characters {
            character.tick();
            if character.is_active() {
                active += 1;
            }
        }
        active
    }

    /// Blit visible characters onto the canvas and render a frame string.
    pub fn get_formatted_output_string(&mut self) -> String {
        self.canvas.clear();
        for character in &self.characters {
            if character.is_visible
                && self.canvas.coord_is_in_canvas(character.motion.current_coord)
            {
                self.canvas
                    .set_cell(character.motion.current_coord, character.current_visual());
            }
        }
        self.canvas.to_frame_string()
    }
}
