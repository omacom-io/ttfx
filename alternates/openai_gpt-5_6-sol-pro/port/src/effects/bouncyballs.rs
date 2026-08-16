
use super::Effect;
use crate::engine::{CharacterId, Path, Terminal, Waypoint};
use crate::utils::{Color, Coord, Style};

const BALL_DELAY: u32 = 7;
const MOVEMENT_SPEED: f64 = 0.25;

const BALL_COLORS: [Color; 5] = [
    Color::rgb(0xd1, 0xf4, 0xa5),
    Color::rgb(0x96, 0xe2, 0xa4),
    Color::rgb(0x5a, 0xcd, 0xa9),
    Color::rgb(0x3a, 0xb8, 0xb0),
    Color::rgb(0x2e, 0x9d, 0xb2),
];

const FINAL_START: (u8, u8, u8) = (0xc1, 0xf8, 0x0a);
const FINAL_END: (u8, u8, u8) = (0x00, 0xd1, 0xff);
const FINAL_GRADIENT_STEPS: usize = 12;

#[derive(Debug, Clone)]
struct PendingBall {
    id: CharacterId,
    start: Coord,
    target: Coord,
    ball_style: Style,
    final_style: Style,
}

#[derive(Debug, Clone)]
struct ActiveBall {
    id: CharacterId,
    target: Coord,
    final_style: Style,
}

#[derive(Debug, Clone)]
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
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

    fn index(&mut self, upper_bound: usize) -> usize {
        if upper_bound <= 1 {
            0
        } else {
            (self.next_u64() % upper_bound as u64) as usize
        }
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for index in (1..values.len()).rev() {
            let swap_index = self.index(index + 1);
            values.swap(index, swap_index);
        }
    }
}

pub struct Bouncyballs;

impl Bouncyballs {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Bouncyballs {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Bouncyballs {
    fn name(&self) -> &str {
        "bouncyballs"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);
        let width = terminal.canvas().width();
        let height = terminal.canvas().height();

        let mut rng = SimpleRng::new(seed_from_input(input));
        let character_data = terminal
            .characters()
            .iter()
            .filter(|character| !character.input_symbol.is_whitespace())
            .map(|character| {
                (
                    character.id,
                    character.position,
                    BALL_COLORS[rng.index(BALL_COLORS.len())],
                )
            })
            .collect::<Vec<_>>();

        let mut pending = Vec::with_capacity(character_data.len());

        for (id, target, ball_color) in character_data {
            let start = Coord::new(rng.index(width) as i32, 0);
            let final_color = final_gradient_color(target, width, height);

            pending.push(PendingBall {
                id,
                start,
                target,
                ball_style: Style::default().with_foreground(ball_color),
                final_style: Style::default().with_foreground(final_color),
            });

            if let Some(character) = terminal.character_mut(id) {
                character.visible = false;
            }
        }

        rng.shuffle(&mut pending);

        if pending.is_empty() {
            return Vec::new();
        }

        let mut active = Vec::<ActiveBall>::new();
        let mut frames = Vec::new();
        let mut launch_delay = 0_u32;

        while !pending.is_empty() || !active.is_empty() {
            if launch_delay == 0 {
                if let Some(ball) = pending.pop() {
                    if let Some(character) = terminal.character_mut(ball.id) {
                        character.visible = true;
                        character.set_position(ball.start);
                        character.set_appearance('●', ball.ball_style);

                        let mut path = Path::with_waypoints(
                            vec![Waypoint::new(ball.start), Waypoint::new(ball.target)],
                            MOVEMENT_SPEED,
                        );
                        path.set_easing(out_bounce);
                        character.motion.activate_path(path);

                        active.push(ActiveBall {
                            id: ball.id,
                            target: ball.target,
                            final_style: ball.final_style,
                        });
                    }

                    launch_delay = BALL_DELAY;
                }
            } else {
                launch_delay -= 1;
            }

            terminal.step();

            let mut index = 0;
            while index < active.len() {
                let finished = terminal
                    .character(active[index].id)
                    .and_then(|character| character.motion.active_path())
                    .map(|path| !path.is_active())
                    .unwrap_or(true);

                if finished {
                    let ball = active.swap_remove(index);

                    if let Some(character) = terminal.character_mut(ball.id) {
                        let symbol = character.input_symbol;
                        character.set_position(ball.target);
                        character.set_appearance(symbol, ball.final_style);
                    }
                } else {
                    index += 1;
                }
            }

            frames.push(terminal.render_frame());
        }

        frames
    }
}

fn out_bounce(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    const N1: f64 = 7.5625;
    const D1: f64 = 2.75;

    if progress < 1.0 / D1 {
        N1 * progress * progress
    } else if progress < 2.0 / D1 {
        let shifted = progress - 1.5 / D1;
        N1 * shifted * shifted + 0.75
    } else if progress < 2.5 / D1 {
        let shifted = progress - 2.25 / D1;
        N1 * shifted * shifted + 0.9375
    } else {
        let shifted = progress - 2.625 / D1;
        N1 * shifted * shifted + 0.984375
    }
}

fn final_gradient_color(coord: Coord, width: usize, height: usize) -> Color {
    let maximum = width.saturating_sub(1) + height.saturating_sub(1);
    let diagonal = coord.x.max(0) as usize + coord.y.max(0) as usize;

    let progress = if maximum == 0 {
        0.0
    } else {
        diagonal.min(maximum) as f64 / maximum as f64
    };

    let intervals = FINAL_GRADIENT_STEPS.saturating_sub(1).max(1);
    let quantized = (progress * intervals as f64).round() / intervals as f64;

    Color::rgb(
        interpolate_channel(FINAL_START.0, FINAL_END.0, quantized),
        interpolate_channel(FINAL_START.1, FINAL_END.1, quantized),
        interpolate_channel(FINAL_START.2, FINAL_END.2, quantized),
    )
}

fn interpolate_channel(start: u8, end: u8, progress: f64) -> u8 {
    let value = start as f64 + (end as f64 - start as f64) * progress.clamp(0.0, 1.0);
    value.round().clamp(0.0, 255.0) as u8
}

fn seed_from_input(input: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;

    for byte in input.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    hash
}
