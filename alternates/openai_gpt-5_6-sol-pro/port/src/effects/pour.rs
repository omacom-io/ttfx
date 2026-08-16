
use std::collections::{BTreeMap, VecDeque};

use super::Effect;
use crate::engine::animation::{CharacterVisual, Frame, Scene};
use crate::engine::character::CharacterId;
use crate::engine::motion::{Path, Waypoint};
use crate::engine::terminal::Terminal;
use crate::utils::easing::in_quad;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Style};

const MOVEMENT_SPEED: f64 = 0.2;
const POUR_GAP: usize = 1;
const COLOR_STEPS: usize = 12;
const STARTING_COLOR: Color = Color::rgb(0x00, 0xc2, 0xff);
const FINAL_GRADIENT: [Color; 3] = [
    Color::rgb(0x8a, 0x00, 0x8a),
    Color::rgb(0x00, 0xd1, 0xff),
    Color::rgb(0xff, 0xff, 0xff),
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Pour;

impl Pour {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Pour {
    fn name(&self) -> &str {
        "pour"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);

        if terminal.characters().is_empty() {
            return Vec::new();
        }

        let canvas_height = terminal.canvas().height();
        let canvas_width = terminal.canvas().width();

        let mut rows: BTreeMap<i32, Vec<CharacterId>> = BTreeMap::new();
        for character in terminal.characters() {
            rows.entry(character.position.y)
                .or_default()
                .push(character.id);
        }

        let mut seed = initial_seed(&terminal);
        let mut pending = VecDeque::new();

        // Pouring downward must fill the rows furthest from the source first.
        for (_, mut row) in rows.into_iter().rev() {
            shuffle(&mut row, &mut seed);
            pending.extend(row);
        }

        for character in terminal.characters_mut() {
            character.visible = false;
        }

        let mut active = Vec::<CharacterId>::new();
        let mut gap_remaining = 0usize;
        let character_count = pending.len();
        let longest_path = canvas_height.saturating_sub(1);
        let max_frames = character_count
            .saturating_mul(POUR_GAP.saturating_add(2))
            .saturating_add(longest_path.saturating_mul(6))
            .saturating_add(COLOR_STEPS)
            .saturating_add(16);

        let mut frames = Vec::new();

        for _ in 0..max_frames {
            if pending.is_empty() && active.is_empty() {
                break;
            }

            if gap_remaining == 0 {
                if let Some(id) = pending.pop_front() {
                    activate_character(&mut terminal, id, canvas_width, canvas_height);
                    active.push(id);
                    gap_remaining = POUR_GAP;
                }
            } else {
                gap_remaining -= 1;
            }

            terminal.step();

            active.retain(|id| {
                terminal.character(*id).is_some_and(|character| {
                    let motion_active = character
                        .motion
                        .active_path()
                        .is_some_and(|path| path.is_active());
                    let animation_active = character
                        .animation
                        .active_scene()
                        .is_some_and(|scene| scene.is_active());

                    motion_active || animation_active
                })
            });

            frames.push(terminal.render_frame());
        }

        frames
    }
}

fn activate_character(
    terminal: &mut Terminal,
    id: CharacterId,
    canvas_width: usize,
    canvas_height: usize,
) {
    let Some(character) = terminal.character_mut(id) else {
        return;
    };

    let destination = character.position;
    let starting_position = Coord::new(destination.x, 0);
    let final_color = gradient_color_for_coord(
        destination,
        canvas_width,
        canvas_height,
        &FINAL_GRADIENT,
    );

    character.position = starting_position;
    character.visible = true;

    let mut path = Path::with_waypoints(
        vec![
            Waypoint::new(starting_position),
            Waypoint::new(destination),
        ],
        MOVEMENT_SPEED,
    );
    path.set_easing(in_quad);
    character.motion.activate_path(path);

    let mut scene = Scene::new(false);
    for step in 0..COLOR_STEPS {
        let denominator = COLOR_STEPS.saturating_sub(1).max(1);
        let progress = step as f64 / denominator as f64;
        let color = interpolate_color(STARTING_COLOR, final_color, progress);
        let style = Style::default().with_foreground(color);

        scene.add_frame(Frame::new(
            CharacterVisual::new(character.input_symbol, style),
            1,
        ));
    }

    character.animation.activate_scene(scene);
}

fn gradient_color_for_coord(
    coord: Coord,
    _canvas_width: usize,
    canvas_height: usize,
    stops: &[Color],
) -> Color {
    if stops.is_empty() {
        return STARTING_COLOR;
    }

    if stops.len() == 1 || canvas_height <= 1 {
        return stops[0];
    }

    let y = coord.y.max(0) as usize;
    let progress = y.min(canvas_height - 1) as f64 / (canvas_height - 1) as f64;
    let scaled = progress * (stops.len() - 1) as f64;
    let segment = (scaled.floor() as usize).min(stops.len() - 2);
    let segment_progress = scaled - segment as f64;

    interpolate_color(stops[segment], stops[segment + 1], segment_progress)
}

fn interpolate_color(start: Color, end: Color, progress: f64) -> Color {
    let progress = progress.clamp(0.0, 1.0);
    let (start_r, start_g, start_b) = color_components(start);
    let (end_r, end_g, end_b) = color_components(end);

    Color::rgb(
        interpolate_channel(start_r, end_r, progress),
        interpolate_channel(start_g, end_g, progress),
        interpolate_channel(start_b, end_b, progress),
    )
}

fn color_components(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb { r, g, b } => (r, g, b),
        Color::Ansi(value) => (value, value, value),
    }
}

fn interpolate_channel(start: u8, end: u8, progress: f64) -> u8 {
    let value = f64::from(start) + (f64::from(end) - f64::from(start)) * progress;
    value.round().clamp(0.0, 255.0) as u8
}

fn initial_seed(terminal: &Terminal) -> u64 {
    let mut seed = 0xcbf2_9ce4_8422_2325_u64;

    for character in terminal.characters() {
        seed ^= u64::from(character.input_symbol as u32);
        seed = seed.wrapping_mul(0x0000_0100_0000_01b3);
        seed ^= u64::from(character.id.0);
        seed = seed.wrapping_mul(0x0000_0100_0000_01b3);
    }

    if seed == 0 {
        1
    } else {
        seed
    }
}

fn shuffle<T>(values: &mut [T], seed: &mut u64) {
    if values.len() < 2 {
        return;
    }

    for index in (1..values.len()).rev() {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;

        let swap_index = (*seed as usize) % (index + 1);
        values.swap(index, swap_index);
    }
}
