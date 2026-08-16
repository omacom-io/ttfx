
use super::Effect;
use crate::engine::{CharacterId, CharacterVisual, Frame, Path, Scene, Terminal, Waypoint};
use crate::utils::easing::{in_quad, out_expo};
use crate::utils::{Color, Coord, Style};

pub struct Crumble;

impl Crumble {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Crumble {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CharacterPhase {
    Waiting,
    Crumbling,
    Falling,
    Hidden,
    Returning,
    Reforming,
    Done,
}

struct CrumbleCharacter {
    id: CharacterId,
    original_position: Coord,
    phase: CharacterPhase,
    fall_speed: f64,
    return_speed: f64,
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
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

    fn range_f64(&mut self, minimum: f64, maximum: f64) -> f64 {
        let ratio = self.next_u64() as f64 / u64::MAX as f64;
        minimum + (maximum - minimum) * ratio
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for index in (1..values.len()).rev() {
            let target = (self.next_u64() % (index as u64 + 1)) as usize;
            values.swap(index, target);
        }
    }
}

fn interpolate_channel(start: u8, end: u8, progress: f64) -> u8 {
    let value = f64::from(start) + (f64::from(end) - f64::from(start)) * progress;
    value.round().clamp(0.0, 255.0) as u8
}

fn final_style(position: Coord, canvas_height: usize) -> Style {
    let progress = if canvas_height <= 1 {
        0.0
    } else {
        (position.y.max(0) as f64 / (canvas_height - 1) as f64).clamp(0.0, 1.0)
    };

    let start = (0x5c, 0xe1, 0xe6);
    let end = (0xff, 0x8c, 0x00);

    Style::default().with_foreground(Color::rgb(
        interpolate_channel(start.0, end.0, progress),
        interpolate_channel(start.1, end.1, progress),
        interpolate_channel(start.2, end.2, progress),
    ))
}

fn crumble_scene(symbol: char, style: &Style) -> Scene {
    let mut scene = Scene::new(false);

    scene.add_frame(Frame::new(
        CharacterVisual::new(symbol, style.clone()),
        1,
    ));
    scene.add_frame(Frame::new(
        CharacterVisual::new('▉', style.clone()),
        1,
    ));
    scene.add_frame(Frame::new(
        CharacterVisual::new('▓', style.clone()),
        1,
    ));
    scene.add_frame(Frame::new(
        CharacterVisual::new('▒', style.clone()),
        1,
    ));
    scene.add_frame(Frame::new(
        CharacterVisual::new('░', style.clone()),
        1,
    ));
    scene.add_frame(Frame::new(
        CharacterVisual::new(' ', style.clone()),
        1,
    ));

    scene
}

fn dust_scene() -> Scene {
    let dust_style = Style::default().with_foreground(Color::rgb(0xb2, 0xa1, 0x8f));
    let mut scene = Scene::new(true);

    scene.add_frame(Frame::new(
        CharacterVisual::new('░', dust_style.clone()),
        2,
    ));
    scene.add_frame(Frame::new(
        CharacterVisual::new('▒', dust_style.clone()),
        1,
    ));
    scene.add_frame(Frame::new(
        CharacterVisual::new('░', dust_style),
        2,
    ));

    scene
}

fn reform_scene(symbol: char, style: &Style) -> Scene {
    let mut scene = Scene::new(false);

    scene.add_frame(Frame::new(
        CharacterVisual::new('░', style.clone()),
        1,
    ));
    scene.add_frame(Frame::new(
        CharacterVisual::new('▒', style.clone()),
        1,
    ));
    scene.add_frame(Frame::new(
        CharacterVisual::new('▓', style.clone()),
        1,
    ));
    scene.add_frame(Frame::new(
        CharacterVisual::new(symbol, style.clone()),
        1,
    ));

    scene
}

fn scene_finished(terminal: &Terminal, id: CharacterId) -> bool {
    terminal
        .character(id)
        .and_then(|character| character.animation.active_scene())
        .map(|scene| scene.is_finished())
        .unwrap_or(true)
}

fn motion_finished(terminal: &Terminal, id: CharacterId) -> bool {
    terminal
        .character(id)
        .and_then(|character| character.motion.active_path())
        .map(|path| !path.is_active())
        .unwrap_or(true)
}

fn restore_final_frame(
    terminal: &mut Terminal,
    characters: &mut [CrumbleCharacter],
    canvas_height: usize,
) {
    for record in characters {
        if let Some(character) = terminal.character_mut(record.id) {
            let style = final_style(record.original_position, canvas_height);
            character.motion.deactivate();
            character.animation.deactivate();
            character.set_position(record.original_position);
            character.set_appearance(character.input_symbol, style);
            character.visible = true;
        }

        record.phase = CharacterPhase::Done;
    }
}

impl Effect for Crumble {
    fn name(&self) -> &str {
        "crumble"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);
        let canvas_height = terminal.canvas().height();

        if terminal.characters().is_empty() {
            return vec![terminal.render_frame()];
        }

        let mut rng = SimpleRng::from_input(input);
        let mut characters = terminal
            .characters()
            .iter()
            .map(|character| CrumbleCharacter {
                id: character.id,
                original_position: character.position,
                phase: CharacterPhase::Waiting,
                fall_speed: rng.range_f64(0.35, 0.75),
                return_speed: rng.range_f64(0.55, 1.0),
            })
            .collect::<Vec<_>>();

        let mut crumble_order = (0..characters.len()).collect::<Vec<_>>();
        rng.shuffle(&mut crumble_order);

        let mut return_order = crumble_order.clone();
        return_order.reverse();

        let mut crumble_cursor = 0;
        let mut return_cursor = 0;
        let mut stage = 0_u8;
        let mut pause_frames = 4_u8;
        let mut tick = 0_usize;
        let mut frames = Vec::new();

        let maximum_frames = characters
            .len()
            .saturating_mul(12)
            .saturating_add(canvas_height.saturating_mul(8))
            .saturating_add(128);

        while frames.len() < maximum_frames {
            if stage == 0 {
                for record in &mut characters {
                    match record.phase {
                        CharacterPhase::Crumbling if scene_finished(&terminal, record.id) => {
                            if let Some(character) = terminal.character_mut(record.id) {
                                character.animation.deactivate();
                                character.set_appearance(
                                    '░',
                                    Style::default()
                                        .with_foreground(Color::rgb(0xb2, 0xa1, 0x8f)),
                                );

                                let destination =
                                    Coord::new(record.original_position.x, canvas_height as i32 - 1);
                                let mut path = Path::with_waypoints(
                                    vec![
                                        Waypoint::new(character.position),
                                        Waypoint::new(destination),
                                    ],
                                    record.fall_speed,
                                );
                                path.set_easing(in_quad);
                                character.motion.activate_path(path);
                            }

                            record.phase = CharacterPhase::Falling;
                        }
                        CharacterPhase::Falling if motion_finished(&terminal, record.id) => {
                            if let Some(character) = terminal.character_mut(record.id) {
                                character.motion.deactivate();
                                character.animation.deactivate();
                                character.visible = false;
                            }

                            record.phase = CharacterPhase::Hidden;
                        }
                        _ => {}
                    }
                }

                if crumble_cursor < crumble_order.len() && tick % 2 == 0 {
                    let record_index = crumble_order[crumble_cursor];
                    crumble_cursor += 1;

                    let record = &mut characters[record_index];
                    if let Some(character) = terminal.character_mut(record.id) {
                        let style = final_style(record.original_position, canvas_height);
                        character
                            .animation
                            .activate_scene(crumble_scene(character.input_symbol, &style));
                    }
                    record.phase = CharacterPhase::Crumbling;
                }

                if crumble_cursor == crumble_order.len()
                    && characters
                        .iter()
                        .all(|record| record.phase == CharacterPhase::Hidden)
                {
                    stage = 1;
                }
            } else if stage == 1 {
                if pause_frames > 0 {
                    pause_frames -= 1;
                } else {
                    stage = 2;
                }
            } else {
                for record in &mut characters {
                    match record.phase {
                        CharacterPhase::Returning if motion_finished(&terminal, record.id) => {
                            if let Some(character) = terminal.character_mut(record.id) {
                                let style = final_style(record.original_position, canvas_height);
                                character.motion.deactivate();
                                character.set_position(record.original_position);
                                character.animation.activate_scene(reform_scene(
                                    character.input_symbol,
                                    &style,
                                ));
                            }

                            record.phase = CharacterPhase::Reforming;
                        }
                        CharacterPhase::Reforming if scene_finished(&terminal, record.id) => {
                            if let Some(character) = terminal.character_mut(record.id) {
                                let style = final_style(record.original_position, canvas_height);
                                character.animation.deactivate();
                                character.set_position(record.original_position);
                                character.set_appearance(character.input_symbol, style);
                                character.visible = true;
                            }

                            record.phase = CharacterPhase::Done;
                        }
                        _ => {}
                    }
                }

                if return_cursor < return_order.len() {
                    let record_index = return_order[return_cursor];
                    return_cursor += 1;

                    let record = &mut characters[record_index];
                    if record.phase == CharacterPhase::Hidden {
                        if let Some(character) = terminal.character_mut(record.id) {
                            let bottom = Coord::new(
                                record.original_position.x,
                                canvas_height as i32 - 1,
                            );

                            character.set_position(bottom);
                            character.visible = true;
                            character.animation.activate_scene(dust_scene());

                            let mut path = Path::with_waypoints(
                                vec![
                                    Waypoint::new(bottom),
                                    Waypoint::new(record.original_position),
                                ],
                                record.return_speed,
                            );
                            path.set_easing(out_expo);
                            character.motion.activate_path(path);
                        }

                        record.phase = CharacterPhase::Returning;
                    }
                }

                if return_cursor == return_order.len()
                    && characters
                        .iter()
                        .all(|record| record.phase == CharacterPhase::Done)
                {
                    restore_final_frame(&mut terminal, &mut characters, canvas_height);
                    frames.push(terminal.render_frame());
                    break;
                }
            }

            terminal.step();
            frames.push(terminal.render_frame());
            tick = tick.saturating_add(1);
        }

        if characters
            .iter()
            .any(|record| record.phase != CharacterPhase::Done)
        {
            restore_final_frame(&mut terminal, &mut characters, canvas_height);
            frames.push(terminal.render_frame());
        }

        frames
    }
}
