//! Spotlights: the text is dimmed while several spotlights search across the
//! canvas, illuminating the characters they pass over. Finally the spotlights
//! converge in the center and expand until the entire text is illuminated.
//!
//! Port of terminaltexteffects/effects/effect_spotlights.py, adapted to the
//! simplified engine available in this crate.

use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::motion::Path;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Simple deterministic xorshift PRNG so the effect needs no external crates.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // Never allow a zero state.
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15 | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform float in [0, 1).
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform float in [lo, hi).
    fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_f64()
    }

    /// Uniform integer in [lo, hi] (inclusive).
    fn randint(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i32
    }
}

/// Distance with doubled row difference to compensate for terminal cell
/// aspect ratio (mirrors `double_row_diff=True` in the Python original).
fn spot_distance(a: Coord, b: Coord) -> f64 {
    let dc = (a.column - b.column) as f64;
    let dr = ((a.row - b.row) * 2) as f64;
    (dc * dc + dr * dr).sqrt()
}

/// Scale a color's brightness by `factor` (0..=1).
fn scale_color(color: Color, factor: f64) -> Color {
    let f = factor.clamp(0.0, 1.0);
    Color::new(
        (color.r as f64 * f).round() as u8,
        (color.g as f64 * f).round() as u8,
        (color.b as f64 * f).round() as u8,
    )
}

/// One roving spotlight: a sequence of search paths traversed in order.
struct Spotlight {
    coord: Coord,
    paths: Vec<Path>,
    path_index: usize,
}

impl Spotlight {
    /// Advance one tick along the current path; returns true while moving.
    fn step(&mut self) -> bool {
        if self.path_index >= self.paths.len() {
            return false;
        }
        let path = &mut self.paths[self.path_index];
        if let Some(coord) = path.step() {
            self.coord = coord;
        }
        if path.is_complete() {
            self.path_index += 1;
        }
        true
    }

    fn is_searching(&self) -> bool {
        self.path_index < self.paths.len()
    }
}

pub struct Spotlights {
    spotlight_count: usize,
    beam_width_ratio: f64,
    beam_falloff: f64,
    search_duration: usize,
    search_speed_min: f64,
    search_speed_max: f64,
    dark_brightness: f64,
    gradient_stops: [&'static str; 3],
}

impl Spotlights {
    pub fn new() -> Self {
        Spotlights {
            spotlight_count: 3,
            beam_width_ratio: 2.0,
            beam_falloff: 0.3,
            search_duration: 750,
            search_speed_min: 0.25,
            search_speed_max: 0.5,
            dark_brightness: 0.2,
            // Default final gradient stops from the Python effect.
            gradient_stops: ["#8A008A", "#00D1FF", "#FFFFFF"],
        }
    }

    fn random_coord(rng: &mut Rng, width: i32, height: i32) -> Coord {
        Coord::new(rng.randint(1, width), rng.randint(1, height))
    }

    /// Find a random coord at least `minimum_distance` away from `last`.
    fn find_coord_at_minimum_distance(
        rng: &mut Rng,
        last: Coord,
        minimum_distance: f64,
        width: i32,
        height: i32,
    ) -> Coord {
        let mut candidate = Self::random_coord(rng, width, height);
        for _ in 0..200 {
            candidate = Self::random_coord(rng, width, height);
            if spot_distance(last, candidate) >= minimum_distance {
                break;
            }
        }
        candidate
    }

    /// Illuminate characters based on spotlight positions and render a frame.
    fn render_frame(
        &self,
        terminal: &mut Terminal,
        illuminated_colors: &[Color],
        spot_coords: &[Coord],
        radius: f64,
    ) -> String {
        let inner = radius * (1.0 - self.beam_falloff);
        let falloff_span = (radius * self.beam_falloff).max(f64::EPSILON);
        for (i, character) in terminal.characters.iter_mut().enumerate() {
            let distance = spot_coords
                .iter()
                .map(|s| spot_distance(*s, character.input_coord))
                .fold(f64::INFINITY, f64::min);
            let brightness = if spot_coords.is_empty() || distance > radius {
                self.dark_brightness
            } else if distance > inner {
                (1.0 - (distance - inner) / falloff_span).max(self.dark_brightness)
            } else {
                1.0
            };
            let color = scale_color(illuminated_colors[i], brightness);
            character.is_visible = true;
            character.animation.current_visual =
                CharacterVisual::new(character.input_symbol, false, ColorPair::fg(color));
        }
        terminal.get_formatted_output_string()
    }
}

impl Default for Spotlights {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Spotlights {
    fn name(&self) -> &str {
        "spotlights"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;
        let center = terminal.canvas.center();

        // Deterministic seed derived from the input text.
        let seed = input
            .bytes()
            .fold(0xC0FFEEu64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        let mut rng = Rng::new(seed);

        // Final (fully illuminated) color per character: a vertical gradient.
        let stops: Vec<Color> = self
            .gradient_stops
            .iter()
            .filter_map(|hex| Color::from_hex(hex))
            .collect();
        let gradient = Gradient::new(&stops, 12);
        let illuminated_colors: Vec<Color> = terminal
            .get_characters()
            .iter()
            .map(|character| {
                let fraction = if height > 1 {
                    (character.input_coord.row - 1) as f64 / (height - 1) as f64
                } else {
                    1.0
                };
                gradient
                    .get_color_at_fraction(fraction)
                    .unwrap_or(Color::new(255, 255, 255))
            })
            .collect();

        // Beam radius derived from canvas size and the beam width ratio.
        let max_dim = width.max(height) as f64;
        let radius = (max_dim / (self.beam_width_ratio * 2.0)).max(2.0);
        let minimum_distance = (max_dim / 4.0).max(1.0);

        // Build the spotlights and their search paths.
        let mut spotlights: Vec<Spotlight> = Vec::new();
        for spot_index in 0..self.spotlight_count {
            let start = Self::random_coord(&mut rng, width, height);
            let mut last_coord = start;
            let mut paths: Vec<Path> = Vec::new();
            for leg in 0..10 {
                let target = Self::find_coord_at_minimum_distance(
                    &mut rng,
                    last_coord,
                    minimum_distance,
                    width,
                    height,
                );
                let speed = rng.uniform(self.search_speed_min, self.search_speed_max);
                let mut path = Path::new(
                    &format!("search_{spot_index}_{leg}"),
                    speed,
                    Some(easing::in_out_quad),
                );
                path.add_waypoint(last_coord);
                path.add_waypoint(target);
                paths.push(path);
                last_coord = target;
            }
            spotlights.push(Spotlight {
                coord: start,
                paths,
                path_index: 0,
            });
        }

        let mut frames: Vec<String> = Vec::new();

        // Initial frame: everything dimmed, spotlights at their start coords.
        let coords: Vec<Coord> = spotlights.iter().map(|s| s.coord).collect();
        frames.push(self.render_frame(&mut terminal, &illuminated_colors, &coords, radius));

        // --- Search phase -------------------------------------------------
        let mut search_frames = 0usize;
        while search_frames < self.search_duration
            && spotlights.iter().any(Spotlight::is_searching)
        {
            for spotlight in &mut spotlights {
                spotlight.step();
            }
            let coords: Vec<Coord> = spotlights.iter().map(|s| s.coord).collect();
            frames.push(self.render_frame(&mut terminal, &illuminated_colors, &coords, radius));
            search_frames += 1;
        }

        // --- Converge phase: all spotlights head to the canvas center ------
        let mut converge_paths: Vec<Path> = spotlights
            .iter()
            .enumerate()
            .map(|(i, spotlight)| {
                let mut path =
                    Path::new(&format!("converge_{i}"), 0.5, Some(easing::in_out_sine));
                path.add_waypoint(spotlight.coord);
                path.add_waypoint(center);
                path
            })
            .collect();
        let mut guard = 0usize;
        while converge_paths.iter().any(|p| !p.is_complete()) && guard < 2000 {
            for (spotlight, path) in spotlights.iter_mut().zip(converge_paths.iter_mut()) {
                if !path.is_complete() {
                    if let Some(coord) = path.step() {
                        spotlight.coord = coord;
                    }
                }
            }
            let coords: Vec<Coord> = spotlights.iter().map(|s| s.coord).collect();
            frames.push(self.render_frame(&mut terminal, &illuminated_colors, &coords, radius));
            guard += 1;
        }

        // --- Expansion phase: the merged beam grows to cover the canvas ----
        let corner = Coord::new(width, height);
        let max_radius = spot_distance(center, corner) + 1.0;
        let expansion_frames = 30usize;
        for step in 1..=expansion_frames {
            let t = step as f64 / expansion_frames as f64;
            let current_radius = radius + (max_radius - radius) * easing::in_out_sine(t);
            frames.push(self.render_frame(
                &mut terminal,
                &illuminated_colors,
                &[center],
                current_radius,
            ));
        }

        // Final held frame: full illumination for every character.
        for (i, character) in terminal.characters.iter_mut().enumerate() {
            character.is_visible = true;
            character.animation.current_visual = CharacterVisual::new(
                character.input_symbol,
                false,
                ColorPair::fg(illuminated_colors[i]),
            );
        }
        frames.push(terminal.get_formatted_output_string());

        frames
    }
}
