
use super::Effect;
use crate::engine::animation::{CharacterVisual, Frame, Scene};
use crate::engine::motion::{Path, Waypoint};
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Style};

pub struct Rain;

impl Rain {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Rain {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Rain {
    fn name(&self) -> &str {
        "rain"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        const RAIN_COLORS: [Color; 4] = [
            Color::rgb(0x00, 0x31, 0x5c),
            Color::rgb(0x00, 0x4c, 0x8f),
            Color::rgb(0x00, 0x75, 0xdb),
            Color::rgb(0x3f, 0x91, 0xd9),
        ];

        let mut terminal = Terminal::from_text(input);
        let character_count = terminal.characters().len();

        if character_count == 0 {
            return vec![terminal.render_frame()];
        }

        let canvas_height = terminal.canvas().height();
        let mut rng = RainRng::new(seed_from_input(input));

        let targets: Vec<Coord> = terminal
            .characters()
            .iter()
            .map(|character| character.position)
            .collect();

        let final_colors: Vec<Color> = targets
            .iter()
            .map(|coord| final_gradient_color(coord.y, canvas_height))
            .collect();

        for character in terminal.characters_mut() {
            character.visible = false;
            character.motion.deactivate();
            character.animation.deactivate();
        }

        let mut pending: Vec<usize> = (0..character_count).collect();
        shuffle(&mut pending, &mut rng);

        // 0 = pending, 1 = falling, 2 = fading, 3 = complete.
        let mut states = vec![0_u8; character_count];
        let mut completed = 0_usize;
        let mut frames = Vec::new();

        let maximum_steps = character_count
            .saturating_mul(4)
            .saturating_add(canvas_height.saturating_mul(20))
            .saturating_add(64);

        for _ in 0..maximum_steps {
            if completed == character_count {
                break;
            }

            let spawn_count = rng.range_usize(1, 3);
            for _ in 0..spawn_count {
                let Some(index) = pending.pop() else {
                    break;
                };

                let target = targets[index];
                let start = Coord::new(target.x, 0);
                let rain_color = RAIN_COLORS[rng.range_usize(0, RAIN_COLORS.len() - 1)];
                let speed = 0.1 + rng.unit_f64() * 0.1;
                let style = Style::default().with_foreground(rain_color);

                let mut path = Path::with_waypoints(
                    vec![Waypoint::new(start), Waypoint::new(target)],
                    speed,
                );
                path.set_easing(easing::in_quad);

                let character = &mut terminal.characters_mut()[index];
                character.visible = true;
                character.set_position(start);
                character.set_appearance(character.input_symbol, style);
                character.motion.activate_path(path);
                states[index] = 1;
            }

            terminal.step();

            for index in 0..character_count {
                if states[index] == 1 {
                    let still_falling = terminal.characters()[index]
                        .motion
                        .active_path()
                        .is_some_and(Path::is_active);

                    if !still_falling {
                        let symbol = terminal.characters()[index].input_symbol;
                        let rain_color = terminal.characters()[index]
                            .style
                            .colors
                            .foreground
                            .unwrap_or(RAIN_COLORS[0]);
                        let final_color = final_colors[index];
                        let scene = make_fade_scene(symbol, rain_color, final_color);

                        let character = &mut terminal.characters_mut()[index];
                        character.set_position(targets[index]);
                        character.set_appearance(
                            symbol,
                            Style::default().with_foreground(rain_color),
                        );
                        character.animation.activate_scene(scene);
                        states[index] = 2;
                    }
                } else if states[index] == 2 {
                    let fade_finished = terminal.characters()[index]
                        .animation
                        .active_scene()
                        .is_some_and(Scene::is_finished);

                    if fade_finished {
                        states[index] = 3;
                        completed += 1;
                    }
                }
            }

            frames.push(terminal.render_frame());
        }

        if completed < character_count {
            for index in 0..character_count {
                let symbol = terminal.characters()[index].input_symbol;
                let character = &mut terminal.characters_mut()[index];
                character.visible = true;
                character.set_position(targets[index]);
                character.motion.deactivate();
                character.animation.deactivate();
                character.set_appearance(
                    symbol,
                    Style::default().with_foreground(final_colors[index]),
                );
            }
            frames.push(terminal.render_frame());
        }

        frames
    }
}

fn make_fade_scene(symbol: char, start: Color, end: Color) -> Scene {
    const FADE_STEPS: u32 = 8;

    let mut scene = Scene::new(false);
    for step in 0..FADE_STEPS {
        let progress = step as f64 / (FADE_STEPS - 1) as f64;
        let color = interpolate_color(start, end, progress);
        let style = Style::default().with_foreground(color);
        scene.add_frame(Frame::new(CharacterVisual::new(symbol, style), 1));
    }
    scene
}

fn final_gradient_color(y: i32, height: usize) -> Color {
    const STOPS: [Color; 3] = [
        Color::rgb(0x8a, 0x00, 0x8a),
        Color::rgb(0x00, 0xd1, 0xff),
        Color::rgb(0xff, 0xff, 0xff),
    ];

    let progress = if height <= 1 {
        0.0
    } else {
        let y = y.clamp(0, height.saturating_sub(1) as i32) as f64;
        1.0 - y / (height - 1) as f64
    };

    let scaled = progress.clamp(0.0, 1.0) * (STOPS.len() - 1) as f64;
    let lower = scaled.floor() as usize;
    let upper = (lower + 1).min(STOPS.len() - 1);
    interpolate_color(STOPS[lower], STOPS[upper], scaled - lower as f64)
}

fn interpolate_color(start: Color, end: Color, progress: f64) -> Color {
    let (start_r, start_g, start_b) = color_rgb(start);
    let (end_r, end_g, end_b) = color_rgb(end);
    let progress = progress.clamp(0.0, 1.0);

    let interpolate = |start: u8, end: u8| {
        (start as f64 + (end as f64 - start as f64) * progress).round() as u8
    };

    Color::rgb(
        interpolate(start_r, end_r),
        interpolate(start_g, end_g),
        interpolate(start_b, end_b),
    )
}

fn color_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb { r, g, b } => (r, g, b),
        Color::Ansi(value) => (value, value, value),
    }
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

fn shuffle<T>(values: &mut [T], rng: &mut RainRng) {
    for index in (1..values.len()).rev() {
        let swap_index = rng.range_usize(0, index);
        values.swap(index, swap_index);
    }
}

struct RainRng {
    state: u64,
}

impl RainRng {
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

    fn unit_f64(&mut self) -> f64 {
        const SCALE: f64 = 1.0 / ((1_u64 << 53) as f64);
        ((self.next_u64() >> 11) as f64) * SCALE
    }

    fn range_usize(&mut self, minimum: usize, maximum: usize) -> usize {
        if minimum >= maximum {
            return minimum;
        }

        minimum + (self.next_u64() as usize % (maximum - minimum + 1))
    }
}
