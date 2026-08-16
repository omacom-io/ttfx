use std::f64::consts::TAU;

use super::Effect;
use crate::engine::{CharacterId, Path, Terminal, Waypoint};
use crate::utils::easing::{out_expo, out_quad};
use crate::utils::{Color, Coord, Style};

const BUBBLE_DELAY: usize = 50;
const BUBBLE_SPEED: f64 = 0.1;
const POP_SPEED: f64 = 0.3;
const RETURN_SPEED: f64 = 0.3;

const BUBBLE_COLORS: [Color; 4] = [
    Color::rgb(0xd3, 0x3a, 0xff),
    Color::rgb(0x43, 0xc2, 0xff),
    Color::rgb(0x2d, 0xff, 0x8a),
    Color::rgb(0xff, 0xf7, 0x00),
];

const POP_COLOR: Color = Color::rgb(0xff, 0xff, 0xff);
const FINAL_COLOR_TOP: Color = Color::rgb(0x43, 0xc2, 0xff);
const FINAL_COLOR_BOTTOM: Color = Color::rgb(0xd3, 0x3a, 0xff);

pub struct Bubbles;

impl Bubbles {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Bubbles {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Bubbles {
    fn name(&self) -> &str {
        "bubbles"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);
        let width = terminal.canvas().width();
        let height = terminal.canvas().height();

        let mut members = terminal
            .characters()
            .iter()
            .filter(|character| !character.input_symbol.is_whitespace())
            .map(|character| Member {
                id: character.id,
                home: character.position,
                symbol: character.input_symbol,
            })
            .collect::<Vec<_>>();

        for character in terminal.characters_mut() {
            character.visible = false;
        }

        if members.is_empty() {
            return Vec::new();
        }

        let mut rng = SimpleRng::new(seed_from_input(input));
        shuffle(&mut members, &mut rng);

        let mut bubbles = Vec::new();
        let mut remaining = members.as_slice();

        while !remaining.is_empty() {
            let maximum = remaining.len().min(20);
            let minimum = maximum.min(5);
            let group_size = if minimum == maximum {
                maximum
            } else {
                rng.range_inclusive(minimum, maximum)
            };

            let (group, rest) = remaining.split_at(group_size);
            remaining = rest;

            let radius = ((group_size as f64 / TAU).ceil() as i32).max(1);
            let start_x = rng.range_i32(0, width.saturating_sub(1) as i32);
            let start_center = Coord::new(start_x, height as i32 + radius + 1);

            let average_home_y = group
                .iter()
                .map(|member| i64::from(member.home.y))
                .sum::<i64>() as f64
                / group.len() as f64;

            let target_x = rng.range_i32(0, width.saturating_sub(1) as i32);
            let target_center = Coord::new(
                target_x,
                average_home_y.round().clamp(0.0, height.saturating_sub(1) as f64) as i32,
            );

            let color = BUBBLE_COLORS[rng.range(0, BUBBLE_COLORS.len())];

            bubbles.push(Bubble {
                members: group.to_vec(),
                radius,
                start_center,
                target_center,
                color,
                phase: BubblePhase::Pending,
            });
        }

        let mut frames = Vec::new();
        let mut next_release = 0usize;
        let dimension_allowance = width
            .saturating_add(height)
            .saturating_mul(20)
            .saturating_add(1_000);
        let max_steps = bubbles
            .len()
            .saturating_mul(BUBBLE_DELAY)
            .saturating_add(dimension_allowance);

        for tick in 0..max_steps {
            if tick >= next_release {
                if let Some(bubble) = bubbles
                    .iter_mut()
                    .find(|bubble| bubble.phase == BubblePhase::Pending)
                {
                    activate_bubble(bubble, &mut terminal);
                    next_release = next_release.saturating_add(BUBBLE_DELAY);
                }
            }

            terminal.step();

            for bubble in &mut bubbles {
                match bubble.phase {
                    BubblePhase::Rising if members_have_stopped(bubble, &terminal) => {
                        begin_pop(bubble, &mut terminal);
                    }
                    BubblePhase::Popping if members_have_stopped(bubble, &terminal) => {
                        begin_return(bubble, &mut terminal, height);
                    }
                    BubblePhase::Returning if members_have_stopped(bubble, &terminal) => {
                        finish_bubble(bubble, &mut terminal, height);
                    }
                    _ => {}
                }
            }

            frames.push(terminal.render_frame());

            if bubbles
                .iter()
                .all(|bubble| bubble.phase == BubblePhase::Done)
            {
                break;
            }
        }

        if bubbles
            .iter()
            .any(|bubble| bubble.phase != BubblePhase::Done)
        {
            for bubble in &mut bubbles {
                finish_bubble(bubble, &mut terminal, height);
            }
            frames.push(terminal.render_frame());
        }

        frames
    }
}

#[derive(Debug, Clone)]
struct Member {
    id: CharacterId,
    home: Coord,
    symbol: char,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BubblePhase {
    Pending,
    Rising,
    Popping,
    Returning,
    Done,
}

#[derive(Debug, Clone)]
struct Bubble {
    members: Vec<Member>,
    radius: i32,
    start_center: Coord,
    target_center: Coord,
    color: Color,
    phase: BubblePhase,
}

fn activate_bubble(bubble: &mut Bubble, terminal: &mut Terminal) {
    let count = bubble.members.len();

    for (index, member) in bubble.members.iter().enumerate() {
        let offset = circle_offset(bubble.radius, index, count);
        let start = bubble.start_center + offset;
        let destination = bubble.target_center + offset;

        if let Some(character) = terminal.character_mut(member.id) {
            character.visible = true;
            character.set_position(start);
            character.set_appearance(
                member.symbol,
                Style::default().with_foreground(bubble.color),
            );

            let mut path = Path::with_waypoints(
                vec![Waypoint::new(start), Waypoint::new(destination)],
                BUBBLE_SPEED,
            );
            path.set_easing(out_quad);
            character.motion.activate_path(path);
        }
    }

    bubble.phase = BubblePhase::Rising;
}

fn begin_pop(bubble: &mut Bubble, terminal: &mut Terminal) {
    let count = bubble.members.len();
    let pop_radius = bubble.radius.saturating_add(3);

    for (index, member) in bubble.members.iter().enumerate() {
        let offset = circle_offset(pop_radius, index, count);
        let destination = bubble.target_center + offset;

        if let Some(character) = terminal.character_mut(member.id) {
            let start = character.position;
            character.set_appearance(
                member.symbol,
                Style::default().with_foreground(POP_COLOR),
            );

            let mut path = Path::with_waypoints(
                vec![Waypoint::new(start), Waypoint::new(destination)],
                POP_SPEED,
            );
            path.set_easing(out_expo);
            character.motion.activate_path(path);
        }
    }

    bubble.phase = BubblePhase::Popping;
}

fn begin_return(bubble: &mut Bubble, terminal: &mut Terminal, height: usize) {
    for member in &bubble.members {
        if let Some(character) = terminal.character_mut(member.id) {
            let start = character.position;
            let color = final_color(member.home.y, height);

            character.set_appearance(member.symbol, Style::default().with_foreground(color));

            let mut path = Path::with_waypoints(
                vec![Waypoint::new(start), Waypoint::new(member.home)],
                RETURN_SPEED,
            );
            path.set_easing(out_expo);
            character.motion.activate_path(path);
        }
    }

    bubble.phase = BubblePhase::Returning;
}

fn finish_bubble(bubble: &mut Bubble, terminal: &mut Terminal, height: usize) {
    for member in &bubble.members {
        if let Some(character) = terminal.character_mut(member.id) {
            character.motion.deactivate();
            character.set_position(member.home);
            character.set_appearance(
                member.symbol,
                Style::default().with_foreground(final_color(member.home.y, height)),
            );
            character.visible = true;
        }
    }

    bubble.phase = BubblePhase::Done;
}

fn members_have_stopped(bubble: &Bubble, terminal: &Terminal) -> bool {
    bubble.members.iter().all(|member| {
        terminal
            .character(member.id)
            .and_then(|character| character.motion.active_path())
            .map(|path| !path.is_active())
            .unwrap_or(true)
    })
}

fn circle_offset(radius: i32, index: usize, count: usize) -> Coord {
    if count == 0 {
        return Coord::ZERO;
    }

    let angle = TAU * index as f64 / count as f64;
    Coord::new(
        (angle.cos() * radius as f64).round() as i32,
        (angle.sin() * radius as f64).round() as i32,
    )
}

fn final_color(row: i32, height: usize) -> Color {
    let denominator = height.saturating_sub(1).max(1) as f64;
    let progress = (row.max(0) as f64 / denominator).clamp(0.0, 1.0);

    interpolate_color(FINAL_COLOR_TOP, FINAL_COLOR_BOTTOM, progress)
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

fn shuffle<T>(values: &mut [T], rng: &mut SimpleRng) {
    for index in (1..values.len()).rev() {
        let other = rng.range_inclusive(0, index);
        values.swap(index, other);
    }
}

#[derive(Debug, Clone)]
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
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

    fn range(&mut self, start: usize, end: usize) -> usize {
        if end <= start {
            return start;
        }

        start + (self.next_u64() as usize % (end - start))
    }

    fn range_inclusive(&mut self, start: usize, end: usize) -> usize {
        if end <= start {
            return start;
        }

        start + (self.next_u64() as usize % (end - start + 1))
    }

    fn range_i32(&mut self, start: i32, end: i32) -> i32 {
        if end <= start {
            return start;
        }

        start + (self.next_u64() % (end - start + 1) as u64) as i32
    }
}
