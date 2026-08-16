use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{self, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Sprays characters from a single origin point onto their input coordinates.
pub struct Spray;

impl Spray {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Spray {
    fn default() -> Self {
        Self::new()
    }
}

const GRADIENT_HEX: [&str; 3] = ["8A008A", "00D1FF", "FFFFFF"];
const GRADIENT_STEPS: usize = 12;
const HOLD_FRAMES: usize = 8;
const TRAVEL_FRAMES: usize = 48;
const VOLUME: f64 = 0.08;

fn hex_color(hex: &str) -> Color {
    let h = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(255);
    Color::rgb(r, g, b)
}

fn ease_out_expo(t: f64) -> f64 {
    if t >= 1.0 {
        1.0
    } else {
        1.0 - 2f64.powf(-10.0 * t)
    }
}

fn lerp_color(a: Color, b: Color, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    let ar = a.r as f64;
    let ag = a.g as f64;
    let ab = a.b as f64;
    let br = b.r as f64;
    let bg = b.g as f64;
    let bb = b.b as f64;
    Color::rgb(
        (ar + (br - ar) * t).round() as u8,
        (ag + (bg - ag) * t).round() as u8,
        (ab + (bb - ab) * t).round() as u8,
    )
}

impl Effect for Spray {
    fn name(&self) -> &str {
        "spray"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_input(input, TerminalConfig::default());
        if terminal.character_count() == 0 {
            return vec![terminal.render_frame()];
        }

        let palette: Vec<Color> = GRADIENT_HEX.iter().map(|h| hex_color(h)).collect();
        let final_gradient = Gradient::new(&palette, GRADIENT_STEPS);

        let (min_row, max_row) = {
            let chars = terminal.get_characters();
            let min_row = chars.iter().map(|c| c.input_coord.row).min().unwrap_or(1);
            let max_row = chars.iter().map(|c| c.input_coord.row).max().unwrap_or(1);
            (min_row, max_row)
        };

        let snapshots: Vec<(CharacterId, String, Coord, Color)> = terminal
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
                (ch.id, ch.input_symbol.clone(), ch.input_coord, color)
            })
            .collect();

        let origin = Coord {
            column: terminal.canvas.right.saturating_sub(1).max(1),
            row: (terminal.canvas.top / 2).max(1),
        };

        // Hide everyone at the origin with a colored glyph so the first frame is styled.
        for (id, symbol, _coord, color) in &snapshots {
            if let Some(ch) = terminal.get_character_mut(*id) {
                ch.motion.current_coord = origin;
                ch.animation
                    .set_appearance(symbol, Some(ColorPair::fg(*color)));
                ch.is_visible = false;
            }
        }

        let n = snapshots.len();
        let per_tick = ((n as f64) * VOLUME).ceil().max(1.0) as usize;
        let launch_span = (n + per_tick - 1) / per_tick;
        let total_anim = launch_span + TRAVEL_FRAMES;

        let mut frames: Vec<String> = Vec::with_capacity(total_anim + HOLD_FRAMES);

        for tick in 0..total_anim {
            let launched = ((tick + 1) * per_tick).min(n);
            for (i, (id, symbol, dest, color)) in snapshots.iter().enumerate() {
                if i >= launched {
                    continue;
                }
                let launch_tick = i / per_tick;
                let age = tick.saturating_sub(launch_tick);
                let t = ease_out_expo((age as f64 / TRAVEL_FRAMES as f64).clamp(0.0, 1.0));
                let coord = if age + 1 >= TRAVEL_FRAMES {
                    *dest
                } else {
                    geometry::lerp_coord(origin, *dest, t)
                };
                let start_color = palette[1];
                let painted = lerp_color(start_color, *color, t);
                if let Some(ch) = terminal.get_character_mut(*id) {
                    ch.motion.current_coord = coord;
                    ch.animation
                        .set_appearance(symbol, Some(ColorPair::fg(painted)));
                    ch.is_visible = true;
                }
            }
            frames.push(terminal.render_frame());
        }

        for (id, symbol, dest, color) in &snapshots {
            if let Some(ch) = terminal.get_character_mut(*id) {
                ch.motion.current_coord = *dest;
                ch.animation
                    .set_appearance(symbol, Some(ColorPair::fg(*color)));
                ch.is_visible = true;
            }
        }
        for _ in 0..HOLD_FRAMES {
            frames.push(terminal.render_frame());
        }

        frames
    }
}
