use super::Effect;
use crate::engine::canvas::Cell;
use crate::engine::terminal::Terminal;

pub struct Wipe;

impl Wipe {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Wipe {
    fn name(&self) -> &str {
        "wipe"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (term_width, term_height) = Terminal::autodetect_size();
        let width = term_width.max(1);
        let height = term_height.max(1);

        let mut terminal = Terminal::from_input(input, width, height);
        let characters = terminal.characters.clone();

        let mut frames = Vec::new();

        // Start with a blank canvas.
        terminal.clear_canvas();
        frames.push(terminal.write_frame());

        let cols = terminal.canvas.width as usize;

        // Reveal the input characters column-by-column from left to right.
        for current_col in 0..cols {
            terminal.clear_canvas();

            for character in &characters {
                let x = character.position.x.round() as u16;
                let y = character.position.y.round() as u16;

                if x <= current_col as u16 {
                    let cell = Cell::new(character.input_symbol.clone(), character.style);
                    terminal.canvas.set_cell(x, y, cell);
                }
            }

            frames.push(terminal.write_frame());
        }

        // If the terminal somehow has no columns, keep the blank frame.
        if cols == 0 {
            frames.push(terminal.write_frame());
        }

        frames
    }
}
