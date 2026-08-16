use super::Effect;

use crate::engine::{Path, Terminal, Waypoint};
use crate::utils::{Color, Coord, Style};

const MOVEMENT_SPEED: f64 = 0.3;
const GRADIENT_STEPS: usize = 12;
const GRADIENT_START: (u8, u8, u8) = (0xff, 0x90, 0x48);
const GRADIENT_END: (u8, u8, u8) = (0xab, 0x9d, 0xff);

pub struct Scattered;

impl Scattered {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Scattered {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Scattered {
    fn name(&self) -> &str {
        "scattered"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);

        if terminal.characters().is_empty() {
            return Vec::new();
        }

        let width = terminal.canvas().width();
        let height = terminal.canvas().height();
        let mut rng = LocalRng::new(seed_from_input(input, width, height));

        for character in terminal.characters_mut() {
            let destination = character.position;
            let start = Coord::new(
                rng.index(width) as i32,
                rng.index(height) as i32,
            );

            character.set_position(start);
            character.visible = true;
            character.set_appearance(
                character.input_symbol,
                Style::default().with_foreground(gradient_color(destination.y, height)),
            );

            let mut path = Path::with_waypoints(
                vec![Waypoint::new(start), Waypoint::new(destination)],
                MOVEMENT_SPEED,
            );
            path.set_easing(in_out_back);
            character.motion.activate_path(path);
        }

        let mut frames = Vec::new();

        while terminal.characters().iter().any(|character| {
            character
                .motion
                .active_path()
                .is_some_and(|path| path.is_active())
        }) {
            terminal.step();
            frames.push(terminal.render_frame());
        }

        frames
    }
}

fn gradient_color(y: i32, height: usize) -> Color {
    let step = if height <= 1 {
        0
    } else {
        let y = y.clamp(0, height.saturating_sub(1) as i32) as usize;
        let distance_from_bottom = height - 1 - y;
        distance_from_bottom * (GRADIENT_STEPS - 1) / (height - 1)
    };

    let denominator = (GRADIENT_STEPS - 1) as u32;
    let step = step as u32;

    Color::rgb(
        interpolate_channel(GRADIENT_START.0, GRADIENT_END.0, step, denominator),
        interpolate_channel(GRADIENT_START.1, GRADIENT_END.1, step, denominator),
        interpolate_channel(GRADIENT_START.2, GRADIENT_END.2, step, denominator),
    )
}

fn interpolate_channel(start: u8, end: u8, step: u32, denominator: u32) -> u8 {
    let start = i64::from(start);
    let difference = i64::from(end) - start;
    let numerator = difference * i64::from(step);
    let denominator = i64::from(denominator.max(1));

    let offset = if numerator >= 0 {
        (numerator + denominator / 2) / denominator
    } else {
        (numerator - denominator / 2) / denominator
    };

    (start + offset).clamp(0, 255) as u8
}

fn in_out_back(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    let c1 = 1.70158;
    let c2 = c1 * 1.525;

    if progress < 0.5 {
        let scaled = 2.0 * progress;
        (scaled * scaled * ((c2 + 1.0) * scaled - c2)) / 2.0
    } else {
        let scaled = 2.0 * progress - 2.0;
        (scaled * scaled * ((c2 + 1.0) * scaled + c2) + 2.0) / 2.0
    }
}

fn seed_from_input(input: &str, width: usize, height: usize) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;

    for byte in input.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    hash ^= width as u64;
    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    hash ^= height as u64;

    hash
}

struct LocalRng {
    state: u64,
}

impl LocalRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9e37_79b9_7f4a_7c15,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state
    }

    fn index(&mut self, upper_bound: usize) -> usize {
        if upper_bound <= 1 {
            0
        } else {
            (self.next_u64() % upper_bound as u64) as usize
        }
    }
}
