use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Prints the input data in a random sequence, one character at a time.
pub struct RandomSequence;

impl RandomSequence {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RandomSequence {
    fn default() -> Self {
        Self::new()
    }
}

const SPEED: f64 = 0.007;
const GRADIENT_STEPS: usize = 12;
const FADE_FRAMES: usize = 56;
const HOLD_FRAMES: usize = 8;
const STOP_HEX: [&str; 3] = ["8A008A", "00D1FF", "FFFFFF"];

fn hex_color(hex: &str) -> Color {
    Color::from_hex(hex).unwrap_or(Color::rgb(255, 255, 255))
}

fn blend(a: Color, b: Color, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    let ah = a.hex();
    let bh = b.hex();
    let parse = |s: &str| -> (u8, u8, u8) {
        let s = s.trim_start_matches('#');
        if s.len() >= 6 {
            let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
            (r, g, b)
        } else {
            (255, 255, 255)
        }
    };
    let (ar, ag, ab) = parse(&ah);
    let (br, bg, bb) = parse(&bh);
    Color::rgb(
        (ar as f64 + (br as f64 - ar as f64) * t).round() as u8,
        (ag as f64 + (bg as f64 - ag as f64) * t).round() as u8,
        (ab as f64 + (bb as f64 - ab as f64) * t).round() as u8,
    )
}

struct Pending {
    id: CharacterId,
    symbol: String,
    coord: Coord,
    final_color: Color,
}

impl Effect for RandomSequence {
    fn name(&self) -> &str {
        "random_sequence"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_input(input, TerminalConfig::default());
        if terminal.character_count() == 0 {
            return vec![terminal.render_frame()];
        }

        let palette: Vec<Color> = STOP_HEX.iter().map(|h| hex_color(h)).collect();
        let final_gradient = Gradient::new(&palette, GRADIENT_STEPS);
        let bg = Color::rgb(0, 0, 0);

        let (min_row, max_row) = {
            let chars = terminal.get_characters();
            let min_row = chars.iter().map(|c| c.input_coord.row).min().unwrap_or(1);
            let max_row = chars.iter().map(|c| c.input_coord.row).max().unwrap_or(1);
            (min_row, max_row)
        };

        let mut pending: Vec<Pending> = terminal
            .get_characters()
            .iter()
            .map(|ch| {
                let progress = if max_row == min_row {
                    0.0
                } else {
                    f64::from(ch.input_coord.row - min_row) / f64::from(max_row - min_row)
                };
                let color = final_gradient
                    .mapped_color(progress)
                    .unwrap_or(palette[0]);
                Pending {
                    id: ch.id,
                    symbol: ch.input_symbol.clone(),
                    coord: ch.input_coord,
                    final_color: color,
                }
            })
            .collect();

        for p in &pending {
            if let Some(ch) = terminal.get_character_mut(p.id) {
                ch.motion.set_coordinate(p.coord);
                ch.animation
                    .set_appearance(&p.symbol, Some(ColorPair::fg(p.final_color)));
                ch.is_visible = false;
            }
        }

        // Deterministic shuffle (LCG) matching a stable reveal order.
        let n = pending.len();
        let mut state: u64 = 0xC0FFEE ^ (n as u64).wrapping_mul(0x9E37);
        for i in (1..n).rev() {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let j = (state as usize) % (i + 1);
            pending.swap(i, j);
        }

        let per_tick = ((SPEED * n as f64) as usize).max(1);
        let mut frames: Vec<String> = Vec::new();
        let mut next = 0usize;
        let mut fading: Vec<(usize, usize)> = Vec::new();

        loop {
            let mut i = 0;
            while i < per_tick && next < pending.len() {
                fading.push((next, 0));
                if let Some(ch) = terminal.get_character_mut(pending[next].id) {
                    ch.is_visible = true;
                }
                next += 1;
                i += 1;
            }

            let mut still: Vec<(usize, usize)> = Vec::new();
            for (idx, age) in fading {
                let p = &pending[idx];
                let t = ((age + 1) as f64 / FADE_FRAMES as f64).min(1.0);
                let color = blend(bg, p.final_color, t);
                if let Some(ch) = terminal.get_character_mut(p.id) {
                    ch.motion.set_coordinate(p.coord);
                    ch.animation
                        .set_appearance(&p.symbol, Some(ColorPair::fg(color)));
                    ch.is_visible = true;
                }
                if age + 1 < FADE_FRAMES {
                    still.push((idx, age + 1));
                }
            }
            fading = still;

            frames.push(terminal.render_frame());
            if next >= pending.len() && fading.is_empty() {
                break;
            }
        }

        for p in &pending {
            if let Some(ch) = terminal.get_character_mut(p.id) {
                ch.motion.set_coordinate(p.coord);
                ch.animation
                    .set_appearance(&p.symbol, Some(ColorPair::fg(p.final_color)));
                ch.is_visible = true;
            }
        }
        for _ in 0..HOLD_FRAMES {
            frames.push(terminal.render_frame());
        }
        frames
    }
}
