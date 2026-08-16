
use super::Effect;
use crate::engine::{CharacterId, Path, Terminal, Waypoint};
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Style};

pub struct Binarypath;

impl Binarypath {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Binarypath {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Binarypath {
    fn name(&self) -> &str {
        "binarypath"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);
        let width = terminal.canvas().width();
        let height = terminal.canvas().height();

        let source_characters = terminal
            .characters()
            .iter()
            .map(|character| {
                (
                    character.id,
                    character.input_symbol,
                    character.position,
                )
            })
            .collect::<Vec<_>>();

        if source_characters.is_empty() {
            return vec![terminal.render_frame()];
        }

        let mut seed = seed_from_input(input);
        let mut groups = Vec::with_capacity(source_characters.len());

        for (source_id, source_symbol, destination) in source_characters {
            let final_style = final_style(destination, width, height);

            if let Some(source) = terminal.character_mut(source_id) {
                source.visible = false;
                source.set_appearance(source_symbol, final_style);
            }

            let route = make_route(destination, width, height, &mut seed);
            let binary_string = format!("{:08b}", source_symbol as u32);
            let mut bit_ids = Vec::with_capacity(binary_string.len());

            for bit in binary_string.chars() {
                let bit_id = terminal.add_character(bit, route[0]);

                if let Some(character) = terminal.character_mut(bit_id) {
                    character.visible = false;
                    character.set_appearance(bit, binary_style(bit));
                }

                bit_ids.push(bit_id);
            }

            groups.push(BinaryGroup {
                source_id,
                bit_ids,
                route,
                next_bit: 0,
                started: false,
                complete: false,
            });
        }

        shuffle(&mut groups, &mut seed);

        let active_limit = ((groups.len() as f64 * 0.05).ceil() as usize).max(1);
        let mut frames = Vec::new();

        while groups.iter().any(|group| !group.complete) {
            let mut active_count = groups
                .iter()
                .filter(|group| group.started && !group.complete)
                .count();

            if active_count < active_limit {
                for group in &mut groups {
                    if active_count >= active_limit {
                        break;
                    }

                    if !group.started && !group.complete {
                        group.started = true;
                        active_count += 1;
                    }
                }
            }

            for group in &mut groups {
                if !group.started || group.complete {
                    continue;
                }

                if let Some(&bit_id) = group.bit_ids.get(group.next_bit) {
                    if let Some(bit) = terminal.character_mut(bit_id) {
                        bit.visible = true;
                        bit.set_position(group.route[0]);

                        let waypoints = group
                            .route
                            .iter()
                            .copied()
                            .map(Waypoint::new)
                            .collect::<Vec<_>>();
                        let path = Path::with_waypoints(waypoints, 1.0);
                        bit.motion.activate_path(path);
                    }

                    group.next_bit += 1;
                }
            }

            terminal.step();

            for group in &mut groups {
                if !group.started || group.complete || group.next_bit < group.bit_ids.len() {
                    continue;
                }

                let travel_complete = group.bit_ids.iter().all(|&bit_id| {
                    terminal
                        .character(bit_id)
                        .and_then(|character| character.motion.active_path())
                        .map(|path| !path.is_active())
                        .unwrap_or(true)
                });

                if travel_complete {
                    for &bit_id in &group.bit_ids {
                        if let Some(bit) = terminal.character_mut(bit_id) {
                            bit.visible = false;
                            bit.motion.deactivate();
                        }
                    }

                    if let Some(source) = terminal.character_mut(group.source_id) {
                        source.visible = true;
                    }

                    group.complete = true;
                }
            }

            frames.push(terminal.render_frame());
        }

        if frames.is_empty() {
            frames.push(terminal.render_frame());
        }

        frames
    }
}

struct BinaryGroup {
    source_id: CharacterId,
    bit_ids: Vec<CharacterId>,
    route: Vec<Coord>,
    next_bit: usize,
    started: bool,
    complete: bool,
}

fn binary_style(bit: char) -> Style {
    let color = if bit == '1' {
        Color::rgb(0, 209, 255)
    } else {
        Color::rgb(138, 0, 138)
    };

    Style::default().with_foreground(color)
}

fn final_style(coord: Coord, width: usize, height: usize) -> Style {
    let maximum = width.saturating_sub(1) + height.saturating_sub(1);
    let progress = if maximum == 0 {
        1.0
    } else {
        let x = coord.x.max(0) as usize;
        let y = coord.y.max(0) as usize;
        (x + y) as f64 / maximum as f64
    };

    let color = if progress < 0.5 {
        interpolate_color(
            Color::rgb(138, 0, 138),
            Color::rgb(0, 209, 255),
            progress * 2.0,
        )
    } else {
        interpolate_color(
            Color::rgb(0, 209, 255),
            Color::rgb(255, 255, 255),
            (progress - 0.5) * 2.0,
        )
    };

    Style::default().with_foreground(color)
}

fn interpolate_color(start: Color, end: Color, progress: f64) -> Color {
    let (
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
    ) = (start, end)
    else {
        return end;
    };

    let progress = progress.clamp(0.0, 1.0);
    let channel = |start: u8, end: u8| {
        (start as f64 + (end as f64 - start as f64) * progress).round() as u8
    };

    Color::rgb(
        channel(start_r, end_r),
        channel(start_g, end_g),
        channel(start_b, end_b),
    )
}

fn make_route(
    destination: Coord,
    width: usize,
    height: usize,
    seed: &mut u64,
) -> Vec<Coord> {
    let right = width.saturating_sub(1) as i32;
    let bottom = height.saturating_sub(1) as i32;
    let edge = next_random(seed) % 4;

    let start = match edge {
        0 => Coord::new(0, random_coordinate(seed, height)),
        1 => Coord::new(right, random_coordinate(seed, height)),
        2 => Coord::new(random_coordinate(seed, width), 0),
        _ => Coord::new(random_coordinate(seed, width), bottom),
    };

    let corner = if edge < 2 {
        Coord::new(destination.x, start.y)
    } else {
        Coord::new(start.x, destination.y)
    };

    vec![start, corner, destination]
}

fn random_coordinate(seed: &mut u64, extent: usize) -> i32 {
    if extent <= 1 {
        0
    } else {
        (next_random(seed) % extent as u64) as i32
    }
}

fn seed_from_input(input: &str) -> u64 {
    let mut seed = 0xcbf2_9ce4_8422_2325_u64;

    for byte in input.bytes() {
        seed ^= u64::from(byte);
        seed = seed.wrapping_mul(0x0000_0100_0000_01b3);
    }

    if seed == 0 {
        0x9e37_79b9_7f4a_7c15
    } else {
        seed
    }
}

fn next_random(seed: &mut u64) -> u64 {
    let mut value = *seed;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *seed = value;
    value
}

fn shuffle<T>(values: &mut [T], seed: &mut u64) {
    for index in (1..values.len()).rev() {
        let swap_index = (next_random(seed) % (index as u64 + 1)) as usize;
        values.swap(index, swap_index);
    }
}
