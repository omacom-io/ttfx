
use super::Effect;
use crate::engine::{CharacterId, Path, Terminal, Waypoint};
use crate::utils::easing::out_expo;
use crate::utils::{Color, Coord, Style};

const SPRAY_VOLUME: f64 = 0.005;
const MOVEMENT_SPEED: f64 = 0.4;

#[derive(Debug, Clone, Copy, Default)]
pub struct Spray;

impl Spray {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Spray {
    fn name(&self) -> &str {
        "spray"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);
        let width = terminal.canvas().width();
        let height = terminal.canvas().height();

        if terminal.characters().is_empty() {
            return Vec::new();
        }

        // The upstream default sprays from the east edge at the canvas midpoint.
        let spray_origin = Coord::new(
            width.saturating_sub(1) as i32,
            height.saturating_sub(1).saturating_div(2) as i32,
        );

        let mut pending: Vec<CharacterId> = terminal
            .characters()
            .iter()
            .map(|character| character.id)
            .collect();

        let total_characters = pending.len();
        let spray_volume =
            ((total_characters as f64 * SPRAY_VOLUME) as usize).max(1);

        let mut rng = SprayRng::from_input(input);
        rng.shuffle(&mut pending);

        for character in terminal.characters_mut() {
            let destination = character.position;
            let style = Style::default().with_foreground(gradient_color(
                destination.y,
                height,
            ));

            character.visible = false;
            character.set_position(spray_origin);
            character.set_appearance(character.input_symbol, style);

            let mut path = Path::new(MOVEMENT_SPEED);
            path.set_easing(out_expo);
            path.add_waypoint(Waypoint::new(spray_origin));
            path.add_waypoint(Waypoint::new(destination));
            character.motion.deactivate();

            // Paths are activated as characters are released from the pending pool.
            // Store no inactive path because Motion only exposes an active-path slot;
            // the same path is reconstructed at activation time.
        }

        let destinations: Vec<(CharacterId, Coord)> = terminal
            .characters()
            .iter()
            .map(|character| {
                // Characters currently sit at the spray origin. Their original grid
                // coordinates are recovered from allocation order below.
                let index = character.id.0 as usize;
                let x = index_to_coord(input, width, index)
                    .map(|coord| coord.x)
                    .unwrap_or(character.position.x);
                let y = index_to_coord(input, width, index)
                    .map(|coord| coord.y)
                    .unwrap_or(character.position.y);
                (character.id, Coord::new(x, y))
            })
            .collect();

        let mut active = Vec::<CharacterId>::new();
        let mut frames = Vec::new();

        while !pending.is_empty() || !active.is_empty() {
            for _ in 0..spray_volume.min(pending.len()) {
                let Some(id) = pending.pop() else {
                    break;
                };

                let destination = destinations
                    .iter()
                    .find_map(|(candidate, coord)| (*candidate == id).then_some(*coord))
                    .unwrap_or(spray_origin);

                if let Some(character) = terminal.character_mut(id) {
                    let mut path = Path::new(MOVEMENT_SPEED);
                    path.set_easing(out_expo);
                    path.add_waypoint(Waypoint::new(spray_origin));
                    path.add_waypoint(Waypoint::new(destination));

                    character.visible = true;
                    character.motion.activate_path(path);
                    active.push(id);
                }
            }

            terminal.step();
            active.retain(|id| {
                terminal
                    .character(*id)
                    .and_then(|character| character.motion.active_path())
                    .is_some_and(Path::is_active)
            });
            frames.push(terminal.render_frame());
        }

        frames
    }
}

fn gradient_color(y: i32, height: usize) -> Color {
    const PURPLE: (u8, u8, u8) = (0x8a, 0x00, 0x8a);
    const CYAN: (u8, u8, u8) = (0x00, 0xd1, 0xff);
    const WHITE: (u8, u8, u8) = (0xff, 0xff, 0xff);

    if height <= 1 {
        return Color::rgb(PURPLE.0, PURPLE.1, PURPLE.2);
    }

    // The source engine's rows increase from bottom to top. This port's rows
    // increase from top to bottom, so invert the vertical progress.
    let y = y.clamp(0, height.saturating_sub(1) as i32) as f64;
    let progress = 1.0 - y / height.saturating_sub(1) as f64;

    if progress <= 0.5 {
        interpolate_color(PURPLE, CYAN, progress * 2.0)
    } else {
        interpolate_color(CYAN, WHITE, (progress - 0.5) * 2.0)
    }
}

fn interpolate_color(
    start: (u8, u8, u8),
    end: (u8, u8, u8),
    progress: f64,
) -> Color {
    fn channel(start: u8, end: u8, progress: f64) -> u8 {
        let value = f64::from(start)
            + (f64::from(end) - f64::from(start)) * progress.clamp(0.0, 1.0);
        value.round().clamp(0.0, 255.0) as u8
    }

    Color::rgb(
        channel(start.0, end.0, progress),
        channel(start.1, end.1, progress),
        channel(start.2, end.2, progress),
    )
}

fn index_to_coord(input: &str, width: usize, target: usize) -> Option<Coord> {
    let lines: Vec<&str> = if input.is_empty() {
        vec![""]
    } else {
        input.lines().collect()
    };

    let mut index = 0usize;

    for (y, line) in lines.iter().enumerate() {
        for (x, _) in line.chars().enumerate() {
            if index == target {
                return Some(Coord::new(x as i32, y as i32));
            }
            index += 1;
        }
    }

    let _ = width;
    None
}

#[derive(Debug, Clone, Copy)]
struct SprayRng {
    state: u64,
}

impl SprayRng {
    fn from_input(input: &str) -> Self {
        let mut state = 0xcbf2_9ce4_8422_2325_u64;

        for byte in input.bytes() {
            state ^= u64::from(byte);
            state = state.wrapping_mul(0x0000_0100_0000_01b3);
        }

        if state == 0 {
            state = 0x9e37_79b9_7f4a_7c15;
        }

        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for upper in (1..values.len()).rev() {
            let index = (self.next_u64() % (upper as u64 + 1)) as usize;
            values.swap(upper, index);
        }
    }
}
