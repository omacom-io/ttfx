use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{self, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Characters disperse into concentric spinning rings, then return home.
pub struct Rings;

impl Rings {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Rings {
    fn default() -> Self {
        Self::new()
    }
}

const RING_HEX: [&str; 3] = ["ab48ff", "e7b2b2", "fffebd"];
const RING_GAP: i32 = 6;
const FORM_FRAMES: usize = 45;
const SPIN_FRAMES: usize = 200;
const DISPERSE_FRAMES: usize = 200;
const HOLD_FRAMES: usize = 12;
const COLOR_INTRO_FRAMES: usize = 50;
const GRADIENT_STEPS: usize = 12;

struct Ring {
    coords: Vec<Coord>,
    color: Color,
    speed: f64,
}

struct Rider {
    id: CharacterId,
    symbol: String,
    input: Coord,
    final_color: Color,
    ring: usize,
    pos: f64,
    clockwise: bool,
}

impl Effect for Rings {
    fn name(&self) -> &str {
        "rings"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_input(input, TerminalConfig::default());
        if terminal.character_count() == 0 {
            return vec![terminal.render_frame()];
        }

        let palette: Vec<Color> = RING_HEX.iter().map(|h| hex_color(h)).collect();
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

        let center = terminal.canvas.center();
        let limit = terminal.canvas.right.max(terminal.canvas.top) / 2;
        let mut rings: Vec<Ring> = Vec::new();
        let mut radius = 1_i32;
        while radius < limit {
            let n_points = (7 * radius).max(1) as usize;
            let coords = geometry::find_coords_on_circle(center, f64::from(radius), n_points, true);
            if coords.len() >= 2 {
                let color = palette[((radius / RING_GAP) as usize) % palette.len()];
                let slot = rings.len() % 5;
                let speed = 0.25 + 0.75 * (slot as f64 / 4.0);
                rings.push(Ring {
                    coords,
                    color,
                    speed,
                });
            }
            radius += RING_GAP;
        }
        if rings.is_empty() {
            let coords = geometry::find_coords_on_circle(center, 2.0, 16, true);
            if coords.len() >= 2 {
                rings.push(Ring {
                    coords,
                    color: palette[0],
                    speed: 0.5,
                });
            }
        }
        if rings.is_empty() {
            for (id, symbol, coord, color) in &snapshots {
                if let Some(ch) = terminal.get_character_mut(*id) {
                    ch.motion.current_coord = *coord;
                    ch.animation
                        .set_appearance(symbol, Some(ColorPair::fg(*color)));
                    ch.is_visible = true;
                }
            }
            return vec![terminal.render_frame()];
        }

        let n_rings = rings.len();
        let mut per_ring = vec![0usize; n_rings];
        for i in 0..snapshots.len() {
            per_ring[i % n_rings] += 1;
        }
        let mut seen = vec![0usize; n_rings];
        let mut riders: Vec<Rider> = Vec::with_capacity(snapshots.len());
        for (i, (id, symbol, input, final_color)) in snapshots.into_iter().enumerate() {
            let ring = i % n_rings;
            let n_coords = rings[ring].coords.len();
            let n_on_ring = per_ring[ring].max(1);
            let k = seen[ring];
            seen[ring] += 1;
            let pos = (k as f64) * (n_coords as f64) / (n_on_ring as f64);
            riders.push(Rider {
                id,
                symbol,
                input,
                final_color,
                ring,
                pos,
                clockwise: i % 2 == 1,
            });
        }

        let mut frames: Vec<String> = Vec::with_capacity(FORM_FRAMES + SPIN_FRAMES + DISPERSE_FRAMES + HOLD_FRAMES);

        // Form: fly from the input layout onto the rings.
        for step in 0..FORM_FRAMES {
            let t = ease_in_out_quad((step + 1) as f64 / FORM_FRAMES as f64);
            for rider in &riders {
                let ring = &rings[rider.ring];
                let home = coord_on_ring(&ring.coords, rider.pos);
                let coord = geometry::lerp_coord(rider.input, home, t);
                let color = blend(rider.final_color, ring.color, t);
                paint(&mut terminal, rider, coord, color);
            }
            frames.push(terminal.render_frame());
        }

        // Spin: characters chase waypoints around their ring.
        for step in 0..SPIN_FRAMES {
            let intro_t = ((step + 1) as f64 / COLOR_INTRO_FRAMES as f64).clamp(0.0, 1.0);
            for rider in &mut riders {
                let ring = &rings[rider.ring];
                if rider.clockwise {
                    rider.pos += ring.speed;
                } else {
                    rider.pos -= ring.speed;
                }
                let coord = coord_on_ring(&ring.coords, rider.pos);
                let color = if step < COLOR_INTRO_FRAMES {
                    blend(rider.final_color, ring.color, intro_t)
                } else {
                    ring.color
                };
                paint(&mut terminal, rider, coord, color);
            }
            frames.push(terminal.render_frame());
        }

        let spin_homes: Vec<Coord> = riders
            .iter()
            .map(|r| coord_on_ring(&rings[r.ring].coords, r.pos))
            .collect();

        // Disperse: ease back to the original input coordinates.
        for step in 0..DISPERSE_FRAMES {
            let linear = (step + 1) as f64 / DISPERSE_FRAMES as f64;
            let eased = ease_in_out_back(linear);
            for (rider, start) in riders.iter().zip(spin_homes.iter()) {
                let ring_color = rings[rider.ring].color;
                let coord = if step + 1 == DISPERSE_FRAMES {
                    rider.input
                } else {
                    geometry::lerp_coord(*start, rider.input, eased)
                };
                let color = if step + 1 == DISPERSE_FRAMES {
                    rider.final_color
                } else {
                    blend(ring_color, rider.final_color, linear)
                };
                paint(&mut terminal, rider, coord, color);
            }
            frames.push(terminal.render_frame());
        }

        for rider in &riders {
            paint(&mut terminal, rider, rider.input, rider.final_color);
        }
        let settled = terminal.render_frame();
        for _ in 0..HOLD_FRAMES {
            frames.push(settled.clone());
        }

        frames
    }
}

fn paint(terminal: &mut Terminal, rider: &Rider, coord: Coord, color: Color) {
    if let Some(ch) = terminal.get_character_mut(rider.id) {
        ch.motion.current_coord = coord;
        ch.animation
            .set_appearance(&rider.symbol, Some(ColorPair::fg(color)));
        ch.is_visible = true;
    }
}

fn coord_on_ring(coords: &[Coord], pos: f64) -> Coord {
    if coords.is_empty() {
        return Coord::new(1, 1);
    }
    let n = coords.len() as f64;
    let wrapped = pos.rem_euclid(n);
    let mut idx = wrapped as usize;
    if idx >= coords.len() {
        idx = 0;
    }
    coords[idx]
}

fn blend(from: Color, to: Color, t: f64) -> Color {
    Gradient::new(&[from, to], 10)
        .mapped_color(t.clamp(0.0, 1.0))
        .unwrap_or(to)
}

fn hex_color(hex: &str) -> Color {
    Color::from_hex(hex).unwrap_or(Color::rgb(255, 255, 255))
}

fn ease_in_out_quad(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

fn ease_in_out_back(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    let c1 = 1.70158;
    let c2 = c1 * 1.525;
    if t < 0.5 {
        ((2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2)) / 2.0
    } else {
        ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (t * 2.0 - 2.0) + c2) + 2.0) / 2.0
    }
}
