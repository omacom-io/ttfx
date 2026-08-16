
use super::Effect;
use crate::engine::{CharacterVisual, Frame, Path, Scene, Terminal, Waypoint};
use crate::utils::{Color, Coord, Style};

pub struct Smoke;

impl Smoke {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Smoke {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Smoke {
    fn name(&self) -> &str {
        "smoke"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);

        if terminal.characters().is_empty() {
            return vec![terminal.render_frame()];
        }

        let width = terminal.canvas().width();
        let height = terminal.canvas().height();
        let targets: Vec<Coord> = terminal
            .characters()
            .iter()
            .map(|character| character.position)
            .collect();

        let final_styles: Vec<Style> = targets
            .iter()
            .map(|target| final_style(*target, width, height))
            .collect();

        let mut rng = SmokeRng::new(hash_input(input));
        let mut pending: Vec<usize> = (0..terminal.characters().len()).collect();
        shuffle(&mut pending, &mut rng);

        for character in terminal.characters_mut() {
            character.visible = false;
        }

        let launch_count = (width / 12).max(1);
        let mut next_pending = 0;
        let mut launch_delay = 0usize;
        let mut frames = Vec::new();
        let mut steps = 0usize;
        let maximum_steps = pending
            .len()
            .saturating_mul(4)
            .saturating_add(height.saturating_mul(4))
            .saturating_add(256);

        while next_pending < pending.len() || has_active_characters(&terminal) {
            if launch_delay == 0 && next_pending < pending.len() {
                let remaining = pending.len() - next_pending;
                let burst = launch_count.min(remaining);

                for _ in 0..burst {
                    let index = pending[next_pending];
                    next_pending += 1;

                    let target = targets[index];
                    let final_style = final_styles[index].clone();
                    let symbol = terminal.characters()[index].input_symbol;

                    let horizontal_drift = rng.range_i32(-2, 3);
                    let vertical_offset = rng.range_i32(2, 6);
                    let start = Coord::new(
                        target.x + horizontal_drift,
                        height as i32 - 1 + vertical_offset,
                    );

                    let distance = start.distance(target);
                    let speed = 0.55 + rng.next_f64() * 0.30;
                    let travel_steps = ((distance / speed).ceil() as usize).max(8);

                    let mut path = Path::new(speed);
                    path.add_waypoint(Waypoint::new(start));
                    path.add_waypoint(Waypoint::new(target));

                    let scene = smoke_scene(
                        symbol,
                        final_style.clone(),
                        travel_steps,
                        rng.range_usize(0, SMOKE_SYMBOLS.len()),
                    );

                    let character = &mut terminal.characters_mut()[index];
                    character.visible = true;
                    character.set_position(start);
                    character.set_appearance(
                        SMOKE_SYMBOLS[0],
                        Style::default().with_foreground(SMOKE_COLORS[0]),
                    );
                    character.motion.activate_path(path);
                    character.animation.activate_scene(scene);
                }

                launch_delay = 1 + rng.range_usize(0, 3);
            } else {
                launch_delay = launch_delay.saturating_sub(1);
            }

            terminal.step();
            frames.push(terminal.render_frame());
            steps += 1;

            if steps >= maximum_steps {
                break;
            }
        }

        for (index, character) in terminal.characters_mut().iter_mut().enumerate() {
            character.visible = true;
            character.motion.deactivate();
            character.animation.deactivate();
            character.set_position(targets[index]);
            character.set_appearance(character.input_symbol, final_styles[index].clone());
        }

        let final_frame = terminal.render_frame();
        if frames.last() != Some(&final_frame) {
            frames.push(final_frame);
        }

        if frames.is_empty() {
            frames.push(terminal.render_frame());
        }

        frames
    }
}

const SMOKE_SYMBOLS: [char; 8] = ['░', '▒', '▓', '█', '▓', '▒', '░', '·'];

const SMOKE_COLORS: [Color; 5] = [
    Color::rgb(72, 72, 72),
    Color::rgb(104, 104, 104),
    Color::rgb(144, 144, 144),
    Color::rgb(184, 184, 184),
    Color::rgb(220, 220, 220),
];

fn smoke_scene(
    final_symbol: char,
    final_style: Style,
    travel_steps: usize,
    symbol_offset: usize,
) -> Scene {
    let mut frames = Vec::with_capacity(travel_steps + 1);

    for step in 0..travel_steps {
        let progress = if travel_steps <= 1 {
            1.0
        } else {
            step as f64 / (travel_steps - 1) as f64
        };

        let symbol_index = (symbol_offset + step) % SMOKE_SYMBOLS.len();
        let color_position = progress * (SMOKE_COLORS.len() - 1) as f64;
        let color_index = color_position.floor() as usize;
        let next_color_index = (color_index + 1).min(SMOKE_COLORS.len() - 1);
        let color_progress = color_position - color_index as f64;
        let color = mix_color(
            SMOKE_COLORS[color_index],
            SMOKE_COLORS[next_color_index],
            color_progress,
        );

        frames.push(Frame::new(
            CharacterVisual::new(
                SMOKE_SYMBOLS[symbol_index],
                Style::default().with_foreground(color),
            ),
            1,
        ));
    }

    frames.push(Frame::new(
        CharacterVisual::new(final_symbol, final_style),
        1,
    ));

    Scene::with_frames(frames, false)
}

fn has_active_characters(terminal: &Terminal) -> bool {
    terminal.characters().iter().any(|character| {
        character
            .motion
            .active_path()
            .is_some_and(|path| path.is_active())
            || character
                .animation
                .active_scene()
                .is_some_and(|scene| scene.is_active())
    })
}

fn final_style(coord: Coord, width: usize, height: usize) -> Style {
    let progress = if height > 1 {
        coord.y.max(0) as f64 / (height - 1) as f64
    } else if width > 1 {
        coord.x.max(0) as f64 / (width - 1) as f64
    } else {
        1.0
    }
    .clamp(0.0, 1.0);

    let color = if progress < 0.5 {
        mix_rgb((138, 0, 138), (0, 209, 255), progress * 2.0)
    } else {
        mix_rgb((0, 209, 255), (255, 255, 255), (progress - 0.5) * 2.0)
    };

    Style::default().with_foreground(color)
}

fn mix_color(start: Color, end: Color, progress: f64) -> Color {
    match (start, end) {
        (
            Color::Rgb {
                r: start_r,
                g: start_g,
                b: start_b,
            },
            Color::Rgb {
                r: end_r,
                g: end_g,
                b: end_b,
            },
        ) => mix_rgb(
            (start_r, start_g, start_b),
            (end_r, end_g, end_b),
            progress,
        ),
        _ if progress < 0.5 => start,
        _ => end,
    }
}

fn mix_rgb(start: (u8, u8, u8), end: (u8, u8, u8), progress: f64) -> Color {
    let progress = progress.clamp(0.0, 1.0);
    let interpolate = |from: u8, to: u8| {
        (from as f64 + (to as f64 - from as f64) * progress)
            .round()
            .clamp(0.0, 255.0) as u8
    };

    Color::rgb(
        interpolate(start.0, end.0),
        interpolate(start.1, end.1),
        interpolate(start.2, end.2),
    )
}

fn hash_input(input: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;

    for byte in input.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    if hash == 0 {
        0x9e3779b97f4a7c15
    } else {
        hash
    }
}

fn shuffle(values: &mut [usize], rng: &mut SmokeRng) {
    for index in (1..values.len()).rev() {
        let swap_index = rng.range_usize(0, index + 1);
        values.swap(index, swap_index);
    }
}

struct SmokeRng {
    state: u64,
}

impl SmokeRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.max(1),
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn next_f64(&mut self) -> f64 {
        let value = self.next_u64() >> 11;
        value as f64 / ((1_u64 << 53) - 1) as f64
    }

    fn range_usize(&mut self, start: usize, end: usize) -> usize {
        if end <= start {
            return start;
        }

        start + (self.next_u64() as usize % (end - start))
    }

    fn range_i32(&mut self, start: i32, end: i32) -> i32 {
        if end <= start {
            return start;
        }

        start + (self.next_u64() % (end - start) as u64) as i32
    }
}
