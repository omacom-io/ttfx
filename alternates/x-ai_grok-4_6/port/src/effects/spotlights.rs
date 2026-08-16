use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{self, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Spotlights search the text area, illuminating characters, then converge and expand.
pub struct Spotlights;

impl Spotlights {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Spotlights {
    fn default() -> Self {
        Self::new()
    }
}

const STOP_HEX: [&str; 3] = ["ab48ff", "e7b2b2", "fffebd"];
const GRADIENT_STEPS: usize = 12;
const BEAM_WIDTH_RATIO: f64 = 2.0;
const BEAM_FALLOFF: f64 = 0.3;
const SEARCH_DURATION: usize = 180;
const SPOTLIGHT_COUNT: usize = 3;
const CONVERGE_FRAMES: usize = 40;
const EXPAND_FRAMES: usize = 50;
const HOLD_FRAMES: usize = 12;
const DARK_FACTOR: f64 = 0.15;

fn hex_color(hex: &str) -> Color {
    Color::from_hex(hex).unwrap_or_else(|| Color::rgb(128, 128, 128))
}

fn ease_in_out_quad(t: f64) -> f64 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

fn ease_in_out_sine(t: f64) -> f64 {
    (-((std::f64::consts::PI * t).cos() - 1.0) / 2.0).clamp(0.0, 1.0)
}

struct Snapshot {
    id: CharacterId,
    symbol: String,
    input: Coord,
    bright: Color,
}

struct Light {
    pos: Coord,
    waypoints: Vec<Coord>,
    wp_index: usize,
    progress: f64,
    speed: f64,
}

impl Effect for Spotlights {
    fn name(&self) -> &str {
        "spotlights"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_input(input, TerminalConfig::default());
        if terminal.character_count() == 0 {
            return vec![terminal.render_frame()];
        }

        let palette: Vec<Color> = STOP_HEX.iter().map(|h| hex_color(h)).collect();
        let final_gradient = Gradient::new(&palette, GRADIENT_STEPS);

        let (min_row, max_row) = {
            let chars = terminal.get_characters();
            let min_row = chars.iter().map(|c| c.input_coord.row).min().unwrap_or(1);
            let max_row = chars.iter().map(|c| c.input_coord.row).max().unwrap_or(1);
            (min_row, max_row)
        };

        let snapshots: Vec<Snapshot> = terminal
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
                Snapshot {
                    id: ch.id,
                    symbol: ch.input_symbol.clone(),
                    input: ch.input_coord,
                    bright: color,
                }
            })
            .collect();

        let canvas_right = terminal.canvas.right;
        let canvas_top = terminal.canvas.top;
        let center = terminal.canvas.center();
        let smallest = canvas_right.min(canvas_top).max(1);
        let illuminate_range = ((smallest as f64 / BEAM_WIDTH_RATIO).min(smallest as f64) as i32).max(1);
        let range_f = illuminate_range as f64;

        let mut lights: Vec<Light> = Vec::new();
        for i in 0..SPOTLIGHT_COUNT {
            let start = Coord::new(
                1 + ((i as i32 * 17 + 3) % canvas_right.max(1)),
                1 + ((i as i32 * 11 + 5) % canvas_top.max(1)),
            );
            let mut waypoints = Vec::new();
            let mut last = start;
            for k in 0..10 {
                let col = 1 + (((last.column + 13 + k as i32 * 7 + i as i32 * 5) % canvas_right.max(1)).abs());
                let row = 1 + (((last.row + 9 + k as i32 * 11 + i as i32 * 3) % canvas_top.max(1)).abs());
                let next = Coord::new(col.max(1).min(canvas_right.max(1)), row.max(1).min(canvas_top.max(1)));
                waypoints.push(next);
                last = next;
            }
            let speed = 0.35 + 0.40 * (i as f64 / (SPOTLIGHT_COUNT.max(1) as f64));
            lights.push(Light {
                pos: start,
                waypoints,
                wp_index: 0,
                progress: 0.0,
                speed,
            });
        }

        for s in &snapshots {
            if let Some(ch) = terminal.get_character_mut(s.id) {
                ch.motion.current_coord = s.input;
                ch.animation
                    .set_appearance(&s.symbol, Some(ColorPair::fg(s.bright.adjust_brightness(DARK_FACTOR))));
                ch.is_visible = true;
            }
        }

        let mut frames: Vec<String> = Vec::new();

        for _ in 0..SEARCH_DURATION {
            for light in &mut lights {
                if light.waypoints.is_empty() {
                    continue;
                }
                let dest = light.waypoints[light.wp_index];
                light.progress += light.speed * 0.15;
                if light.progress >= 1.0 {
                    light.pos = dest;
                    light.progress = 0.0;
                    light.wp_index = (light.wp_index + 1) % light.waypoints.len();
                } else {
                    let t = ease_in_out_quad(light.progress);
                    light.pos = geometry::lerp_coord(light.pos, dest, t);
                }
            }
            paint_illumination(
                &mut terminal,
                &snapshots,
                &lights,
                range_f,
                illuminate_range,
            );
            frames.push(terminal.render_frame());
        }

        let start_pos: Vec<Coord> = lights.iter().map(|l| l.pos).collect();
        for step in 0..CONVERGE_FRAMES {
            let t = ease_in_out_sine((step + 1) as f64 / CONVERGE_FRAMES as f64);
            for (i, light) in lights.iter_mut().enumerate() {
                light.pos = geometry::lerp_coord(start_pos[i], center, t);
            }
            paint_illumination(
                &mut terminal,
                &snapshots,
                &lights,
                range_f,
                illuminate_range,
            );
            frames.push(terminal.render_frame());
        }

        for step in 0..EXPAND_FRAMES {
            let t = (step + 1) as f64 / EXPAND_FRAMES as f64;
            let expand = range_f + t * (smallest as f64);
            paint_illumination(&mut terminal, &snapshots, &lights, expand, expand as i32);
            frames.push(terminal.render_frame());
        }

        for s in &snapshots {
            if let Some(ch) = terminal.get_character_mut(s.id) {
                ch.motion.current_coord = s.input;
                ch.animation
                    .set_appearance(&s.symbol, Some(ColorPair::fg(s.bright)));
                ch.is_visible = true;
            }
        }
        for _ in 0..HOLD_FRAMES {
            frames.push(terminal.render_frame());
        }

        frames
    }
}

fn paint_illumination(
    terminal: &mut Terminal,
    snapshots: &[Snapshot],
    lights: &[Light],
    range_f: f64,
    _range_i: i32,
) {
    let falloff_start = range_f * (1.0 - BEAM_FALLOFF);
    for s in snapshots {
        let mut min_d = f64::MAX;
        for light in lights {
            let d = geometry::find_length_of_line(light.pos, s.input);
            if d < min_d {
                min_d = d;
            }
        }
        let color = if min_d <= falloff_start {
            s.bright
        } else if min_d >= range_f || BEAM_FALLOFF <= 0.0 {
            s.bright.adjust_brightness(DARK_FACTOR.max(0.2))
        } else {
            let factor = (1.0 - (min_d - falloff_start) / (range_f * BEAM_FALLOFF)).max(0.2);
            s.bright.adjust_brightness(factor)
        };
        if let Some(ch) = terminal.get_character_mut(s.id) {
            ch.motion.current_coord = s.input;
            ch.animation
                .set_appearance(&s.symbol, Some(ColorPair::fg(color)));
            ch.is_visible = true;
        }
    }
}
