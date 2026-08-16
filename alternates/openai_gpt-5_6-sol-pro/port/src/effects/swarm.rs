use super::Effect;

use crate::engine::animation::{CharacterVisual, Frame, Scene};
use crate::engine::character::CharacterId;
use crate::engine::motion::{Path, Waypoint};
use crate::engine::terminal::Terminal;
use crate::utils::easing::in_out_sine;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Style};

const SWARM_COLORS: [Color; 3] = [
    Color::rgb(0x31, 0xa0, 0xd4),
    Color::rgb(0x8a, 0x00, 0x8a),
    Color::rgb(0x00, 0xd4, 0x0b),
];
const FLASH_COLOR: Color = Color::rgb(0xff, 0xff, 0xff);
const SWARM_SIZE: f64 = 0.1;
const SWARM_COORDINATION: f64 = 0.8;
const MOVEMENT_SPEED: f64 = 0.55;
const LAUNCH_INTERVAL: usize = 6;

pub struct Swarm;

impl Swarm {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Swarm {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Swarm {
    fn name(&self) -> &str {
        "swarm"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);
        let width = terminal.canvas().width() as i32;
        let height = terminal.canvas().height() as i32;

        if terminal.characters().is_empty() {
            return vec![terminal.render_frame()];
        }

        let mut rng = SwarmRng::new(seed_from_input(input));
        let mut ids: Vec<CharacterId> = terminal
            .characters()
            .iter()
            .map(|character| character.id)
            .collect();
        rng.shuffle(&mut ids);

        let swarm_size = ((ids.len() as f64 * SWARM_SIZE) as usize).max(1);
        let mut pending = Vec::new();

        for (swarm_index, members) in ids.chunks(swarm_size).enumerate() {
            let swarm_color = SWARM_COLORS[rng.index(SWARM_COLORS.len())];
            let spawn = random_coord(&mut rng, width, height);
            let waypoint_count = 2 + rng.index(3);

            let mut coordinated_waypoints = Vec::with_capacity(waypoint_count);
            for _ in 0..waypoint_count {
                coordinated_waypoints.push(random_coord(&mut rng, width, height));
            }

            let mut plans = Vec::with_capacity(members.len());

            for &id in members {
                let Some(character) = terminal.character(id) else {
                    continue;
                };

                let destination = character.position;
                let symbol = character.input_symbol;
                let mut waypoints = Vec::with_capacity(waypoint_count + 2);
                waypoints.push(Waypoint::new(spawn));

                for &coordinated in &coordinated_waypoints {
                    let coord = if rng.next_f64() < SWARM_COORDINATION {
                        jitter_coord(&mut rng, coordinated, width, height)
                    } else {
                        random_coord(&mut rng, width, height)
                    };
                    waypoints.push(Waypoint::new(coord));
                }

                waypoints.push(Waypoint::new(destination));

                let mut path = Path::with_waypoints(
                    waypoints,
                    MOVEMENT_SPEED + rng.next_f64() * 0.2,
                );
                path.set_easing(in_out_sine);

                let mut scene = Scene::new(true);
                let swarm_style = Style::default().with_foreground(swarm_color);
                let flash_style = Style::default().with_foreground(FLASH_COLOR);

                scene.add_frame(Frame::new(
                    CharacterVisual::new(symbol, swarm_style.clone()),
                    5 + rng.index(4) as u32,
                ));
                scene.add_frame(Frame::new(
                    CharacterVisual::new(symbol, flash_style),
                    1,
                ));
                scene.add_frame(Frame::new(
                    CharacterVisual::new(symbol, swarm_style),
                    3 + rng.index(4) as u32,
                ));

                plans.push(CharacterPlan {
                    id,
                    spawn,
                    destination,
                    symbol,
                    path,
                    scene,
                });
            }

            pending.push(SwarmPlan {
                launch_step: swarm_index * LAUNCH_INTERVAL,
                characters: plans,
            });
        }

        for character in terminal.characters_mut() {
            character.visible = false;
        }

        let diagonal = (f64::from(width).hypot(f64::from(height))).ceil() as usize;
        let maximum_steps = diagonal
            .saturating_mul(20)
            .saturating_add(pending.len().saturating_mul(LAUNCH_INTERVAL))
            .saturating_add(128);

        let mut frames = Vec::new();
        let mut next_swarm = 0usize;

        for step in 0..maximum_steps {
            while next_swarm < pending.len() && pending[next_swarm].launch_step <= step {
                for plan in &pending[next_swarm].characters {
                    if let Some(character) = terminal.character_mut(plan.id) {
                        character.set_position(plan.spawn);
                        character.visible = true;
                        character.animation.activate_scene(plan.scene.clone());
                        character.motion.activate_path(plan.path.clone());
                    }
                }
                next_swarm += 1;
            }

            terminal.step();

            for swarm in pending.iter().take(next_swarm) {
                for plan in &swarm.characters {
                    let finished = terminal
                        .character(plan.id)
                        .and_then(|character| character.motion.active_path())
                        .map(|path| !path.is_active())
                        .unwrap_or(true);

                    if finished {
                        let final_color =
                            final_gradient_color(plan.destination, width, height);
                        if let Some(character) = terminal.character_mut(plan.id) {
                            character.set_position(plan.destination);
                            character.motion.deactivate();
                            character.animation.deactivate();
                            character.set_appearance(
                                plan.symbol,
                                Style::default().with_foreground(final_color),
                            );
                        }
                    }
                }
            }

            frames.push(terminal.render_frame());

            let all_launched = next_swarm == pending.len();
            let any_active = terminal.characters().iter().any(|character| {
                character
                    .motion
                    .active_path()
                    .map(|path| path.is_active())
                    .unwrap_or(false)
            });

            if all_launched && !any_active {
                break;
            }
        }

        if frames.is_empty() {
            frames.push(terminal.render_frame());
        }

        frames
    }
}

#[derive(Clone)]
struct CharacterPlan {
    id: CharacterId,
    spawn: Coord,
    destination: Coord,
    symbol: char,
    path: Path,
    scene: Scene,
}

struct SwarmPlan {
    launch_step: usize,
    characters: Vec<CharacterPlan>,
}

fn random_coord(rng: &mut SwarmRng, width: i32, height: i32) -> Coord {
    Coord::new(
        rng.range_i32(0, width.max(1)),
        rng.range_i32(0, height.max(1)),
    )
}

fn jitter_coord(rng: &mut SwarmRng, coord: Coord, width: i32, height: i32) -> Coord {
    let x = (coord.x + rng.range_i32(-2, 3)).clamp(0, width.saturating_sub(1));
    let y = (coord.y + rng.range_i32(-1, 2)).clamp(0, height.saturating_sub(1));
    Coord::new(x, y)
}

fn final_gradient_color(coord: Coord, width: i32, height: i32) -> Color {
    let maximum = (width.saturating_sub(1) + height.saturating_sub(1)).max(1);
    let progress = f64::from(coord.x + coord.y) / f64::from(maximum);

    if progress <= 0.5 {
        interpolate_color(SWARM_COLORS[0], SWARM_COLORS[1], progress * 2.0)
    } else {
        interpolate_color(
            SWARM_COLORS[1],
            SWARM_COLORS[2],
            (progress - 0.5) * 2.0,
        )
    }
}

fn interpolate_color(start: Color, end: Color, progress: f64) -> Color {
    let progress = progress.clamp(0.0, 1.0);

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
        ) => Color::rgb(
            interpolate_channel(start_r, end_r, progress),
            interpolate_channel(start_g, end_g, progress),
            interpolate_channel(start_b, end_b, progress),
        ),
        _ => start,
    }
}

fn interpolate_channel(start: u8, end: u8, progress: f64) -> u8 {
    let value = f64::from(start) + (f64::from(end) - f64::from(start)) * progress;
    value.round().clamp(0.0, 255.0) as u8
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

struct SwarmRng {
    state: u64,
}

impl SwarmRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
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
        let value = self.next_u64() >> 11;
        value as f64 / ((1_u64 << 53) as f64)
    }

    fn index(&mut self, length: usize) -> usize {
        if length <= 1 {
            0
        } else {
            (self.next_u64() % length as u64) as usize
        }
    }

    fn range_i32(&mut self, start: i32, end: i32) -> i32 {
        if end <= start {
            return start;
        }

        start + self.index((end - start) as usize) as i32
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for index in (1..values.len()).rev() {
            let other = self.index(index + 1);
            values.swap(index, other);
        }
    }
}
