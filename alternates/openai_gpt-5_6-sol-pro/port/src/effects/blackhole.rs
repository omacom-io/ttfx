use std::collections::HashSet;

use super::Effect;
use crate::engine::{CharacterId, Path, Terminal, Waypoint};
use crate::utils::easing::{in_expo, in_out_sine, out_expo, out_quad};
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Style};
use crate::utils::EasingFn;

pub struct Blackhole;

impl Blackhole {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Blackhole {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Blackhole {
    fn name(&self) -> &str {
        "blackhole"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);
        let mut frames = Vec::new();

        let ids: Vec<CharacterId> = terminal
            .characters()
            .iter()
            .map(|character| character.id)
            .collect();

        if ids.is_empty() {
            frames.push(terminal.render_frame());
            return frames;
        }

        let width = terminal.canvas().width();
        let height = terminal.canvas().height();
        let center = Coord::new(
            (width.saturating_sub(1) / 2) as i32,
            (height.saturating_sub(1) / 2) as i32,
        );

        let original_positions: Vec<(CharacterId, Coord)> = terminal
            .characters()
            .iter()
            .map(|character| (character.id, character.position))
            .collect();

        let seed = input
            .bytes()
            .fold(0x9e37_79b9_7f4a_7c15_u64, |state, byte| {
                state
                    .wrapping_mul(0x100_0000_01b3)
                    .wrapping_add(u64::from(byte) + 1)
            });
        let mut rng = SmallRng::new(seed);

        let radius_limit = width.min(height).saturating_sub(1) / 2;
        let radius = radius_limit.max(1) as i32;
        let desired_ring_size =
            ((2.0 * std::f64::consts::PI * f64::from(radius)).round() as usize).max(1);
        let ring_size = desired_ring_size.min(ids.len());
        let ring_ids = ids[ids.len() - ring_size..].to_vec();
        let ring_set: HashSet<CharacterId> = ring_ids.iter().copied().collect();
        let ring_positions = circle_positions(center, radius, ring_size, width, height);

        let star_symbols = ['.', '`', '\'', ',', '*'];
        let star_colors = [
            Color::rgb(255, 204, 0),
            Color::rgb(255, 153, 0),
            Color::rgb(255, 102, 0),
            Color::rgb(255, 255, 255),
        ];

        for id in &ids {
            let position = random_coord(&mut rng, width, height);
            let symbol = star_symbols[rng.index(star_symbols.len())];
            let color = star_colors[rng.index(star_colors.len())];

            if let Some(character) = terminal.character_mut(*id) {
                character.set_position(position);
                character.visible = true;
                character.set_appearance(symbol, colored_style(color));
            }
        }

        frames.push(terminal.render_frame());

        for (index, id) in ring_ids.iter().enumerate() {
            let speed = 0.55 + rng.unit() * 0.25;
            activate_move(
                &mut terminal,
                *id,
                ring_positions[index],
                speed,
                in_out_sine,
            );
        }

        run_until_still(
            &mut terminal,
            &ring_ids,
            phase_limit(width, height, 4.0, 40),
            &mut frames,
        );

        let blackhole_color = Color::rgb(255, 255, 255);
        for (index, id) in ring_ids.iter().enumerate() {
            if let Some(character) = terminal.character_mut(*id) {
                character.visible = true;
                character.set_appearance('*', colored_style(blackhole_color));

                let mut path = Path::new(0.45);
                path.set_looping(true);
                path.set_easing(in_out_sine);

                for offset in 0..=ring_positions.len() {
                    let position_index = (index + offset) % ring_positions.len();
                    path.add_waypoint(Waypoint::new(ring_positions[position_index]));
                }

                character.motion.activate_path(path);
            }
        }

        let mut pending: Vec<CharacterId> = ids
            .iter()
            .copied()
            .filter(|id| !ring_set.contains(id))
            .collect();
        shuffle(&mut pending, &mut rng);

        let mut consuming = HashSet::new();
        let launch_count = (pending.len() / 40).max(1);
        let consume_limit = phase_limit(width, height, 8.0, pending.len() + 100);

        for _ in 0..consume_limit {
            for _ in 0..launch_count {
                let Some(id) = pending.pop() else {
                    break;
                };

                let speed = 0.17 + rng.unit() * 0.13;
                activate_move(&mut terminal, id, center, speed, in_expo);
                consuming.insert(id);
            }

            terminal.step();

            let completed: Vec<CharacterId> = consuming
                .iter()
                .copied()
                .filter(|id| !character_is_moving(&terminal, *id))
                .collect();

            for id in completed {
                consuming.remove(&id);
                if let Some(character) = terminal.character_mut(id) {
                    character.visible = false;
                    character.set_position(center);
                }
            }

            frames.push(terminal.render_frame());

            if pending.is_empty() && consuming.is_empty() {
                break;
            }
        }

        for id in &ring_ids {
            if let Some(character) = terminal.character_mut(*id) {
                character.motion.deactivate();
            }
            activate_move(&mut terminal, *id, center, 0.3, in_expo);
        }

        run_until_still(
            &mut terminal,
            &ring_ids,
            phase_limit(width, height, 6.0, 40),
            &mut frames,
        );

        for id in &ring_ids {
            if let Some(character) = terminal.character_mut(*id) {
                character.set_position(center);
                character.set_appearance('*', colored_style(blackhole_color));
            }
        }

        for pulse in 0..8 {
            let color = if pulse % 2 == 0 {
                Color::rgb(255, 255, 255)
            } else {
                Color::rgb(255, 153, 0)
            };

            for id in &ring_ids {
                if let Some(character) = terminal.character_mut(*id) {
                    character.set_appearance('*', colored_style(color));
                }
            }

            frames.push(terminal.render_frame());
        }

        let explosion_colors = [
            Color::rgb(255, 255, 255),
            Color::rgb(255, 204, 0),
            Color::rgb(255, 102, 0),
            Color::rgb(255, 51, 0),
        ];

        for id in &ids {
            let destination = random_coord(&mut rng, width, height);
            let color = explosion_colors[rng.index(explosion_colors.len())];
            let symbol = star_symbols[rng.index(star_symbols.len())];

            if let Some(character) = terminal.character_mut(*id) {
                character.visible = true;
                character.set_position(center);
                character.set_appearance(symbol, colored_style(color));
            }

            activate_move(
                &mut terminal,
                *id,
                destination,
                0.55 + rng.unit() * 0.35,
                out_expo,
            );
        }

        run_until_still(
            &mut terminal,
            &ids,
            phase_limit(width, height, 5.0, 80),
            &mut frames,
        );

        for (id, original_position) in &original_positions {
            activate_move(&mut terminal, *id, *original_position, 0.8, out_quad);
        }

        let home_limit = phase_limit(width, height, 4.0, 80);
        for _ in 0..home_limit {
            terminal.step();

            for (id, original_position) in &original_positions {
                if !character_is_moving(&terminal, *id) {
                    let color = final_color(*original_position, center, width, height);
                    if let Some(character) = terminal.character_mut(*id) {
                        character.set_position(*original_position);
                        character.set_appearance(
                            character.input_symbol,
                            colored_style(color),
                        );
                    }
                }
            }

            frames.push(terminal.render_frame());

            if !ids
                .iter()
                .any(|id| character_is_moving(&terminal, *id))
            {
                break;
            }
        }

        for (id, original_position) in original_positions {
            let color = final_color(original_position, center, width, height);
            if let Some(character) = terminal.character_mut(id) {
                character.motion.deactivate();
                character.visible = true;
                character.set_position(original_position);
                character.set_appearance(character.input_symbol, colored_style(color));
            }
        }

        frames.push(terminal.render_frame());
        frames
    }
}

fn activate_move(
    terminal: &mut Terminal,
    id: CharacterId,
    destination: Coord,
    speed: f64,
    easing: EasingFn,
) {
    let Some(start) = terminal.character(id).map(|character| character.position) else {
        return;
    };

    let mut path = Path::new(speed);
    path.set_easing(easing);
    path.add_waypoint(Waypoint::new(start));
    path.add_waypoint(Waypoint::new(destination));

    if let Some(character) = terminal.character_mut(id) {
        character.motion.activate_path(path);
    }
}

fn character_is_moving(terminal: &Terminal, id: CharacterId) -> bool {
    terminal
        .character(id)
        .and_then(|character| character.motion.active_path())
        .is_some_and(|path| path.is_active())
}

fn run_until_still(
    terminal: &mut Terminal,
    ids: &[CharacterId],
    limit: usize,
    frames: &mut Vec<String>,
) {
    for _ in 0..limit {
        terminal.step();
        frames.push(terminal.render_frame());

        if !ids
            .iter()
            .any(|id| character_is_moving(terminal, *id))
        {
            break;
        }
    }
}

fn circle_positions(
    center: Coord,
    radius: i32,
    count: usize,
    width: usize,
    height: usize,
) -> Vec<Coord> {
    let max_x = width.saturating_sub(1) as i32;
    let max_y = height.saturating_sub(1) as i32;

    (0..count.max(1))
        .map(|index| {
            let angle =
                std::f64::consts::TAU * index as f64 / count.max(1) as f64;
            let x = center.x + (angle.cos() * f64::from(radius)).round() as i32;
            let y = center.y + (angle.sin() * f64::from(radius)).round() as i32;
            Coord::new(x.clamp(0, max_x), y.clamp(0, max_y))
        })
        .collect()
}

fn random_coord(rng: &mut SmallRng, width: usize, height: usize) -> Coord {
    Coord::new(rng.index(width.max(1)) as i32, rng.index(height.max(1)) as i32)
}

fn colored_style(color: Color) -> Style {
    Style::default().with_foreground(color)
}

fn final_color(
    position: Coord,
    center: Coord,
    width: usize,
    height: usize,
) -> Color {
    let max_distance = ((width.max(1) as f64).powi(2)
        + (height.max(1) as f64).powi(2))
    .sqrt()
    .max(1.0);
    let progress = (position.distance(center) / max_distance).clamp(0.0, 1.0);

    if progress < 0.5 {
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
    }
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
        return start;
    };

    let progress = progress.clamp(0.0, 1.0);
    let interpolate = |from: u8, to: u8| {
        (f64::from(from) + (f64::from(to) - f64::from(from)) * progress)
            .round()
            .clamp(0.0, 255.0) as u8
    };

    Color::rgb(
        interpolate(start_r, end_r),
        interpolate(start_g, end_g),
        interpolate(start_b, end_b),
    )
}

fn phase_limit(width: usize, height: usize, multiplier: f64, minimum: usize) -> usize {
    let diagonal =
        ((width.max(1) as f64).powi(2) + (height.max(1) as f64).powi(2)).sqrt();
    ((diagonal * multiplier).ceil() as usize).max(minimum)
}

fn shuffle<T>(values: &mut [T], rng: &mut SmallRng) {
    for index in (1..values.len()).rev() {
        let swap_index = rng.index(index + 1);
        values.swap(index, swap_index);
    }
}

struct SmallRng {
    state: u64,
}

impl SmallRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0xa5a5_5a5a_d3c1_b2e7
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

    fn unit(&mut self) -> f64 {
        self.next_u64() as f64 / u64::MAX as f64
    }

    fn index(&mut self, upper_bound: usize) -> usize {
        if upper_bound <= 1 {
            0
        } else {
            (self.next_u64() % upper_bound as u64) as usize
        }
    }
}
