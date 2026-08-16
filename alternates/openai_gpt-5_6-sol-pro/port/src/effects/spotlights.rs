use super::Effect;

use crate::engine::Terminal;
use crate::utils::graphics::{Color, Style};

const SPOTLIGHT_COLOR: Rgb = Rgb::new(255, 255, 255);
const DARK_COLOR: Rgb = Rgb::new(8, 8, 12);
const SPOTLIGHT_WIDTH: f64 = 10.0;
const SEARCH_FRAMES: usize = 100;
const CONVERGE_FRAMES: usize = 30;
const EXPAND_FRAMES: usize = 35;
const SPOTLIGHT_COUNT: usize = 3;

const FINAL_GRADIENT: [Rgb; 3] = [
    Rgb::new(171, 72, 255),
    Rgb::new(231, 178, 178),
    Rgb::new(255, 254, 189),
];

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

impl Effect for Spotlights {
    fn name(&self) -> &str {
        "spotlights"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);

        if terminal.characters().is_empty() {
            return vec![terminal.render_frame()];
        }

        let width = terminal.canvas().width();
        let height = terminal.canvas().height();
        let center_x = (width.saturating_sub(1)) as f64 / 2.0;
        let center_y = (height.saturating_sub(1)) as f64 / 2.0;

        let mut rng = SmallRng::new(seed_from_input(input));
        let mut spotlights = Vec::with_capacity(SPOTLIGHT_COUNT);

        for index in 0..SPOTLIGHT_COUNT {
            let x = if SPOTLIGHT_COUNT == 1 {
                center_x
            } else {
                index as f64 * width.saturating_sub(1) as f64
                    / (SPOTLIGHT_COUNT - 1) as f64
            };
            let y = if index % 2 == 0 {
                0.0
            } else {
                height.saturating_sub(1) as f64
            };

            let target_x = random_axis(&mut rng, width);
            let target_y = random_axis(&mut rng, height);
            let duration = travel_duration(x, y, target_x, target_y);

            spotlights.push(Spotlight {
                x,
                y,
                start_x: x,
                start_y: y,
                target_x,
                target_y,
                elapsed: 0,
                duration,
            });
        }

        set_all_color(&mut terminal, DARK_COLOR);

        let mut frames =
            Vec::with_capacity(1 + SEARCH_FRAMES + CONVERGE_FRAMES + EXPAND_FRAMES + 1);
        frames.push(terminal.render_frame());

        for _ in 0..SEARCH_FRAMES {
            for spotlight in &mut spotlights {
                spotlight.advance(&mut rng, width, height);
            }

            apply_search_lighting(&mut terminal, &spotlights);
            frames.push(terminal.render_frame());
        }

        let convergence_origins: Vec<(f64, f64)> = spotlights
            .iter()
            .map(|spotlight| (spotlight.x, spotlight.y))
            .collect();

        for step in 1..=CONVERGE_FRAMES {
            let raw_progress = step as f64 / CONVERGE_FRAMES as f64;
            let progress = ease_in_out_sine(raw_progress);

            for (spotlight, (origin_x, origin_y)) in
                spotlights.iter_mut().zip(convergence_origins.iter())
            {
                spotlight.x = lerp(*origin_x, center_x, progress);
                spotlight.y = lerp(*origin_y, center_y, progress);
            }

            apply_search_lighting(&mut terminal, &spotlights);
            frames.push(terminal.render_frame());
        }

        let maximum_radius = {
            let horizontal = width as f64;
            let vertical = height as f64 * 2.0;
            horizontal.hypot(vertical) + SPOTLIGHT_WIDTH
        };

        for step in 1..=EXPAND_FRAMES {
            let raw_progress = step as f64 / EXPAND_FRAMES as f64;
            let progress = ease_out_quad(raw_progress);
            let radius = SPOTLIGHT_WIDTH * 0.5 + maximum_radius * progress;

            apply_expanding_light(
                &mut terminal,
                center_x,
                center_y,
                radius,
                SPOTLIGHT_WIDTH * 0.75,
                width,
                height,
            );
            frames.push(terminal.render_frame());
        }

        apply_final_gradient(&mut terminal, width, height);
        frames.push(terminal.render_frame());
        frames
    }
}

#[derive(Clone, Copy)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

impl Rgb {
    const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    fn mix(self, other: Self, progress: f64) -> Self {
        let progress = progress.clamp(0.0, 1.0);

        Self {
            r: mix_channel(self.r, other.r, progress),
            g: mix_channel(self.g, other.g, progress),
            b: mix_channel(self.b, other.b, progress),
        }
    }

    fn color(self) -> Color {
        Color::rgb(self.r, self.g, self.b)
    }
}

struct Spotlight {
    x: f64,
    y: f64,
    start_x: f64,
    start_y: f64,
    target_x: f64,
    target_y: f64,
    elapsed: usize,
    duration: usize,
}

impl Spotlight {
    fn advance(&mut self, rng: &mut SmallRng, width: usize, height: usize) {
        if self.elapsed >= self.duration {
            self.start_x = self.x;
            self.start_y = self.y;
            self.target_x = random_axis(rng, width);
            self.target_y = random_axis(rng, height);
            self.elapsed = 0;
            self.duration = travel_duration(
                self.start_x,
                self.start_y,
                self.target_x,
                self.target_y,
            );
        }

        self.elapsed += 1;
        let raw_progress = self.elapsed as f64 / self.duration.max(1) as f64;
        let progress = ease_in_out_sine(raw_progress);

        self.x = lerp(self.start_x, self.target_x, progress);
        self.y = lerp(self.start_y, self.target_y, progress);
    }
}

fn apply_search_lighting(terminal: &mut Terminal, spotlights: &[Spotlight]) {
    for character in terminal.characters_mut() {
        let x = character.position.x as f64;
        let y = character.position.y as f64;
        let mut intensity: f64 = 0.0;

        for spotlight in spotlights {
            let dx = x - spotlight.x;
            let dy = (y - spotlight.y) * 2.0;
            let distance = dx.hypot(dy);
            let light = (1.0 - distance / SPOTLIGHT_WIDTH).clamp(0.0, 1.0);
            intensity = intensity.max(light * light);
        }

        let color = DARK_COLOR.mix(SPOTLIGHT_COLOR, intensity);
        character.set_appearance(character.input_symbol, foreground_style(color));
    }
}

fn apply_expanding_light(
    terminal: &mut Terminal,
    center_x: f64,
    center_y: f64,
    radius: f64,
    feather: f64,
    width: usize,
    height: usize,
) {
    for character in terminal.characters_mut() {
        let dx = character.position.x as f64 - center_x;
        let dy = (character.position.y as f64 - center_y) * 2.0;
        let distance = dx.hypot(dy);
        let intensity = ((radius - distance) / feather.max(1.0)).clamp(0.0, 1.0);
        let final_color = gradient_color(character.position.x, character.position.y, width, height);
        let color = DARK_COLOR.mix(final_color, ease_out_quad(intensity));

        character.set_appearance(character.input_symbol, foreground_style(color));
    }
}

fn apply_final_gradient(terminal: &mut Terminal, width: usize, height: usize) {
    for character in terminal.characters_mut() {
        let color = gradient_color(character.position.x, character.position.y, width, height);
        character.set_appearance(character.input_symbol, foreground_style(color));
    }
}

fn set_all_color(terminal: &mut Terminal, color: Rgb) {
    for character in terminal.characters_mut() {
        character.set_appearance(character.input_symbol, foreground_style(color));
    }
}

fn foreground_style(color: Rgb) -> Style {
    Style::default().with_foreground(color.color())
}

fn gradient_color(x: i32, y: i32, width: usize, height: usize) -> Rgb {
    let center_x = width.saturating_sub(1) as f64 / 2.0;
    let center_y = height.saturating_sub(1) as f64 / 2.0;
    let dx = x as f64 - center_x;
    let dy = (y as f64 - center_y) * 2.0;

    let corner_x = center_x.max(1.0);
    let corner_y = (center_y * 2.0).max(1.0);
    let maximum_distance = corner_x.hypot(corner_y).max(1.0);
    let progress = (dx.hypot(dy) / maximum_distance).clamp(0.0, 1.0);

    sample_gradient(&FINAL_GRADIENT, progress)
}

fn sample_gradient(stops: &[Rgb], progress: f64) -> Rgb {
    if stops.is_empty() {
        return SPOTLIGHT_COLOR;
    }

    if stops.len() == 1 {
        return stops[0];
    }

    let scaled = progress.clamp(0.0, 1.0) * (stops.len() - 1) as f64;
    let index = (scaled.floor() as usize).min(stops.len() - 2);
    let local_progress = scaled - index as f64;

    stops[index].mix(stops[index + 1], local_progress)
}

fn mix_channel(start: u8, end: u8, progress: f64) -> u8 {
    let value = start as f64 + (end as f64 - start as f64) * progress;
    value.round().clamp(0.0, 255.0) as u8
}

fn travel_duration(start_x: f64, start_y: f64, end_x: f64, end_y: f64) -> usize {
    let dx = end_x - start_x;
    let dy = (end_y - start_y) * 2.0;
    (dx.hypot(dy) / 0.45).ceil().max(8.0) as usize
}

fn random_axis(rng: &mut SmallRng, length: usize) -> f64 {
    if length <= 1 {
        0.0
    } else {
        rng.next_f64() * length.saturating_sub(1) as f64
    }
}

fn lerp(start: f64, end: f64, progress: f64) -> f64 {
    start + (end - start) * progress.clamp(0.0, 1.0)
}

fn ease_in_out_sine(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    -((std::f64::consts::PI * progress).cos() - 1.0) / 2.0
}

fn ease_out_quad(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    1.0 - (1.0 - progress) * (1.0 - progress)
}

fn seed_from_input(input: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;

    for byte in input.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    if hash == 0 {
        0x9e37_79b9_7f4a_7c15
    } else {
        hash
    }
}

struct SmallRng {
    state: u64,
}

impl SmallRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9e37_79b9_7f4a_7c15
            } else {
                seed
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }

    fn next_f64(&mut self) -> f64 {
        const SCALE: f64 = 1.0 / ((1_u64 << 53) as f64);
        ((self.next_u64() >> 11) as f64) * SCALE
    }
}
