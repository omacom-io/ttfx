use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Python `effect_colorshift` defaults (travel off, one cycle, 5 frames/step).
const GRADIENT_STEPS: usize = 12;
const GRADIENT_FRAMES: usize = 5;
const CYCLES: usize = 1;
const TRAVEL: bool = true;
const SKIP_EMPTY: bool = false;

const STOP_HEX: [&str; 7] = [
    "e81416", "ffa500", "faeb36", "79c314", "487de7", "4b369d", "70369d",
];

pub struct Colorshift;

impl Colorshift {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Colorshift {
    fn name(&self) -> &str {
        "colorshift"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        if term.character_count() == 0 {
            return vec![term.render_frame()];
        }

        let stops: Vec<Color> = STOP_HEX.iter().filter_map(|h| Color::from_hex(h)).collect();
        let gradient = Gradient::new(&stops, GRADIENT_STEPS);
        let spectrum: Vec<Color> = gradient.spectrum().to_vec();
        if spectrum.is_empty() {
            term.show_all();
            return vec![term.render_frame()];
        }

        let n = spectrum.len();
        let (left, right, bottom, top) = text_extents(&term);
        let offsets: Vec<usize> = term
            .get_characters()
            .iter()
            .map(|ch| {
                if TRAVEL {
                    travel_index(ch.input_coord.column, ch.input_coord.row, left, right, bottom, top, n)
                } else {
                    0
                }
            })
            .collect();

        term.show_all();

        let total_steps = n.saturating_mul(CYCLES.max(1));
        let mut frames = Vec::with_capacity(total_steps.saturating_mul(GRADIENT_FRAMES).max(1));

        for step in 0..total_steps {
            for _ in 0..GRADIENT_FRAMES {
                for (i, ch) in term.get_characters_mut().iter_mut().enumerate() {
                    if SKIP_EMPTY && ch.input_symbol.chars().all(char::is_whitespace) {
                        continue;
                    }
                    let idx = (offsets[i] + step) % n;
                    ch.is_visible = true;
                    ch.animation
                        .set_appearance(&ch.input_symbol, Some(ColorPair::fg(spectrum[idx])));
                }
                frames.push(term.render_frame());
            }
        }

        if frames.is_empty() {
            frames.push(term.render_frame());
        }
        frames
    }
}

fn text_extents(term: &Terminal) -> (i32, i32, i32, i32) {
    let chars = term.get_characters();
    let left = chars.iter().map(|c| c.input_coord.column).min().unwrap_or(1);
    let right = chars.iter().map(|c| c.input_coord.column).max().unwrap_or(1);
    let bottom = chars.iter().map(|c| c.input_coord.row).min().unwrap_or(1);
    let top = chars.iter().map(|c| c.input_coord.row).max().unwrap_or(1);
    (left, right, bottom, top)
}

/// Horizontal travel: rotate the spectrum by column, matching Python `--travel`.
fn travel_index(
    column: i32,
    _row: i32,
    left: i32,
    right: i32,
    _bottom: i32,
    _top: i32,
    n: usize,
) -> usize {
    if n == 0 {
        return 0;
    }
    let span = (right - left).max(1) as usize;
    let pos = (column - left).max(0) as usize;
    (pos * n / span) % n
}
