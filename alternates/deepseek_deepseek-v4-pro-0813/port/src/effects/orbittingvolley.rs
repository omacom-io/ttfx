use super::Effect;
use crate::engine::canvas::{Canvas, Cell, CellStyle};
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Gradient};

pub struct Orbittingvolley;

impl Orbittingvolley {
    pub fn new() -> Self {
        Self
    }
}

struct PlacedChar {
    symbol: String,
    input: Coord,
}

fn parse_input(input: &str) -> (Vec<PlacedChar>, u16, u16) {
    let mut chars = Vec::new();
    let mut max_cols = 0u16;
    let mut rows = 0u16;

    for (line_index, line) in input.lines().enumerate() {
        let y = line_index as u16;
        rows = rows.max(y + 1);
        let line_cols = line.chars().count() as u16;
        if line_cols > max_cols {
            max_cols = line_cols;
        }
        for (x, ch) in line.chars().enumerate() {
            chars.push(PlacedChar {
                symbol: ch.to_string(),
                input: Coord::new(x as f32, y as f32),
            });
        }
    }

    if input.is_empty() {
        max_cols = 1;
        rows = 1;
    }

    (chars, max_cols.max(1), rows.max(1))
}

fn draw_at(
    canvas: &mut Canvas,
    position: Coord,
    symbol: &str,
    style: CellStyle,
    width: u16,
    height: u16,
) {
    let x = position.x.round().clamp(0.0, width as f32 - 1.0) as u16;
    let y = position.y.round().clamp(0.0, height as f32 - 1.0) as u16;
    canvas.set_cell(x, y, Cell::new(symbol, style));
}

const TOTAL_FRAMES: usize = 90;
const LAUNCH_FRACTION: f32 = 0.5;
const LAUNCHER_SYMBOL: &str = "●";

impl Effect for Orbittingvolley {
    fn name(&self) -> &str {
        "orbittingvolley"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (chars, width, height) = parse_input(input);
        if chars.is_empty() {
            return vec![Canvas::new(width, height).render_frame()];
        }

        let color_gradient = Gradient::new()
            .add_stop(0.0, Color::CYAN)
            .add_stop(0.5, Color::YELLOW)
            .add_stop(1.0, Color::MAGENTA);

        let launcher_style = CellStyle::new(Color::GREEN, Color::BLACK);
        let mut frames = Vec::with_capacity(TOTAL_FRAMES);

        for frame_index in 0..TOTAL_FRAMES {
            let global_t = if TOTAL_FRAMES <= 1 {
                1.0
            } else {
                frame_index as f32 / (TOTAL_FRAMES - 1) as f32
            };

            let mut canvas = Canvas::new(width, height);

            // The launcher travels along the top edge while it has characters to fire.
            if global_t <= LAUNCH_FRACTION {
                let launcher_progress = easing::ease_in_out_quad(global_t / LAUNCH_FRACTION);
                let launcher_x =
                    (launcher_progress * (width.saturating_sub(1) as f32)).round() as u16;
                draw_at(
                    &mut canvas,
                    Coord::new(launcher_x as f32, 0.0),
                    LAUNCHER_SYMBOL,
                    launcher_style,
                    width,
                    height,
                );
            }

            for (index, placed_char) in chars.iter().enumerate() {
                let launch_t = if chars.len() <= 1 {
                    0.0
                } else {
                    (index as f32 / (chars.len() - 1) as f32) * LAUNCH_FRACTION
                };

                if global_t <= launch_t {
                    continue;
                }

                let raw_flight = (global_t - launch_t) / (1.0 - launch_t).max(0.0001);
                let flight_t = easing::ease_out_cubic(raw_flight.min(1.0));

                let launcher_progress_at_launch = {
                    let raw_progress = launch_t / LAUNCH_FRACTION;
                    easing::ease_in_out_quad(raw_progress.min(1.0))
                };
                let start_x =
                    (launcher_progress_at_launch * (width.saturating_sub(1) as f32)).round();
                let start = Coord::new(start_x, 0.0);
                let position = start.lerp(placed_char.input, flight_t);

                let fg = color_gradient.color_at(flight_t);
                let character_style = CellStyle::new(fg, Color::BLACK);
                draw_at(
                    &mut canvas,
                    position,
                    &placed_char.symbol,
                    character_style,
                    width,
                    height,
                );
            }

            frames.push(canvas.render_frame());
        }

        frames
    }
}
