use crate::engine::canvas::{Canvas, Cell, CellStyle};
use crate::engine::character::EffectCharacter;
use crate::utils::geometry::Coord;

#[derive(Clone, Debug)]
pub struct TerminalConfig {
    pub width: u16,
    pub height: u16,
    pub default_style: CellStyle,
}

impl TerminalConfig {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            default_style: CellStyle::default(),
        }
    }
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

#[derive(Debug)]
pub struct Terminal {
    pub config: TerminalConfig,
    pub canvas: Canvas,
    pub characters: Vec<EffectCharacter>,
}

impl Terminal {
    pub fn new(width: u16, height: u16) -> Self {
        let config = TerminalConfig::new(width, height);
        Self {
            canvas: Canvas::new(width, height),
            config,
            characters: Vec::new(),
        }
    }

    pub fn from_input(input: &str, width: u16, height: u16) -> Self {
        let mut terminal = Self::new(width, height);
        let mut x = 0u16;
        let mut y = 0u16;

        for ch in input.chars() {
            if ch == '\n' {
                y = y.saturating_add(1);
                x = 0;
                continue;
            }

            if y >= height {
                break;
            }

            if x >= width {
                x = 0;
                y = y.saturating_add(1);
                if y >= height {
                    break;
                }
            }

            let id = terminal.characters.len() as u32;
            let position = Coord::new(x as f32, y as f32);
            let character = EffectCharacter::new(id, ch.to_string(), position);
            terminal.characters.push(character);

            let style = terminal.config.default_style;
            terminal.canvas.set_cell(x, y, Cell::new(ch.to_string(), style));
            x += 1;
        }

        terminal
    }

    pub fn set_character_visibility(&mut self, character_id: u32, visible: bool) {
        if let Some(character) = self.characters.iter_mut().find(|c| c.id == character_id) {
            character.set_visibility(visible);
        }
    }

    pub fn write_frame(&self) -> String {
        self.canvas.render_frame()
    }

    pub fn clear_canvas(&mut self) {
        self.canvas.clear();
    }

    pub fn autodetect_size() -> (u16, u16) {
        crossterm::terminal::size().unwrap_or((80, 24))
    }
}
