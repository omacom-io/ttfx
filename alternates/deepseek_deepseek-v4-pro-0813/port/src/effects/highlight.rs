use super::Effect;
use crate::engine::terminal::Terminal;
use crate::utils::graphics::Color;

pub struct Highlight;

impl Highlight {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Highlight {
    fn name(&self) -> &str {
        "highlight"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (width, height) = Terminal::autodetect_size();
        let terminal = Terminal::from_input(input, width, height);

        let beam_width = 5_usize;
        let trail = beam_width as i32 - 1;
        let mut frames = Vec::new();

        for beam_col in -trail..=(width as i32 + trail) {
            frames.push(render_highlight_frame(&terminal, beam_col, trail));
        }

        frames
    }
}

fn render_highlight_frame(terminal: &Terminal, beam_col: i32, trail: i32) -> String {
    let mut out = String::new();

    // Move to the top-left and clear before drawing the frame.
    out.push_str("\x1b[2J\x1b[H");

    for y in 0..terminal.canvas.height {
        for x in 0..terminal.canvas.width {
            let symbol = terminal
                .canvas
                .get(x, y)
                .map(|cell| cell.symbol.clone())
                .unwrap_or_else(|| " ".to_string());

            let (fg, bg) = if (x as i32) >= beam_col - trail && (x as i32) <= beam_col {
                (Color::BLACK, Color::YELLOW)
            } else {
                (Color::WHITE, Color::BLACK)
            };

            out.push_str(&format!(
                "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m{}",
                fg.r, fg.g, fg.b, bg.r, bg.g, bg.b, symbol
            ));
        }
        out.push_str("\x1b[0m\n");
    }

    out
}
