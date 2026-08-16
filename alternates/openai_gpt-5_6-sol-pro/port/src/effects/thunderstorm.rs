use super::Effect;
use crate::engine::{CharacterId, Terminal};
use crate::utils::{Color, Coord, Style};

const FORMATION_FRAMES: usize = 10;
const STORM_FRAMES: usize = 56;
const DISSIPATION_FRAMES: usize = 10;
const FADE_FRAMES: usize = 12;

const DARK_TEXT: Color = Color::rgb(42, 45, 64);
const CLOUD_COLOR: Color = Color::rgb(70, 75, 96);
const RAIN_COLOR: Color = Color::rgb(80, 130, 190);
const LIGHTNING_COLOR: Color = Color::rgb(245, 248, 255);

pub struct Thunderstorm;

impl Thunderstorm {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Thunderstorm {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Thunderstorm {
    fn name(&self) -> &str {
        "thunderstorm"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::from_text(input);
        let width = terminal.canvas().width();
        let height = terminal.canvas().height();
        let width_i32 = width.min(i32::MAX as usize) as i32;
        let height_i32 = height.min(i32::MAX as usize) as i32;

        let input_ids: Vec<CharacterId> = terminal
            .characters()
            .iter()
            .map(|character| character.id)
            .collect();
        let mut glow = vec![0_u8; input_ids.len()];
        let mut rng = StormRng::new(seed_from_input(input));

        for id in &input_ids {
            if let Some(character) = terminal.character_mut(*id) {
                character.set_appearance(
                    character.input_symbol,
                    Style::default().with_foreground(DARK_TEXT),
                );
            }
        }

        let cloud_count = (width / 2 + width % 2).min(512).max(1);
        let mut clouds = Vec::with_capacity(cloud_count);

        for index in 0..cloud_count {
            let target_x = if cloud_count == 1 {
                0
            } else {
                ((index * width.saturating_sub(1)) / (cloud_count - 1)) as i32
            };
            let starts_left = index % 2 == 0;
            let start_x = if starts_left {
                target_x - FORMATION_FRAMES as i32
            } else {
                target_x + FORMATION_FRAMES as i32
            };
            let symbol = match index % 4 {
                0 => '~',
                1 => '-',
                2 => '=',
                _ => '~',
            };
            let id = terminal.add_character(symbol, Coord::new(start_x, 0));

            if let Some(character) = terminal.character_mut(id) {
                character.visible = true;
                character.set_appearance(
                    symbol,
                    Style::default().with_foreground(CLOUD_COLOR),
                );
            }

            clouds.push(Cloud {
                id,
                target_x,
                starts_left,
                symbol,
            });
        }

        let area = width.saturating_mul(height);
        let rain_count = (area / 3)
            .max(width / 2)
            .max(1)
            .min(512);
        let mut rain = Vec::with_capacity(rain_count);

        for index in 0..rain_count {
            let x = rng.range(width) as i32;
            let y = rng.range(height) as i32;
            let speed = 1 + (rng.range(2) as i32);
            let symbol = match index % 3 {
                0 => '|',
                1 => '/',
                _ => '\\',
            };
            let id = terminal.add_character(symbol, Coord::new(x, y));

            if let Some(character) = terminal.character_mut(id) {
                character.visible = false;
                character.set_appearance(
                    symbol,
                    Style::default().with_foreground(RAIN_COLOR),
                );
            }

            rain.push(RainDrop {
                id,
                x,
                y,
                speed,
                symbol,
            });
        }

        let mut frames = Vec::with_capacity(
            FORMATION_FRAMES + STORM_FRAMES + DISSIPATION_FRAMES + FADE_FRAMES + 1,
        );

        for frame in 0..FORMATION_FRAMES {
            let remaining = (FORMATION_FRAMES - frame - 1) as i32;

            for cloud in &clouds {
                let x = if cloud.starts_left {
                    cloud.target_x - remaining
                } else {
                    cloud.target_x + remaining
                };

                if let Some(character) = terminal.character_mut(cloud.id) {
                    character.set_position(Coord::new(x, 0));
                    character.visible = true;
                }
            }

            frames.push(terminal.render_frame());
        }

        let strike_frames = [7_usize, 25, 41, 50];
        let mut lightning_ids = Vec::new();
        let mut flash_remaining = 0_u8;

        for storm_frame in 0..STORM_FRAMES {
            if flash_remaining == 0 && !lightning_ids.is_empty() {
                hide_characters(&mut terminal, &lightning_ids);
                lightning_ids.clear();
            }

            if strike_frames.contains(&storm_frame) {
                hide_characters(&mut terminal, &lightning_ids);
                lightning_ids.clear();

                let mut x = rng.range(width) as i32;

                for y in 0..height_i32.max(1) {
                    let previous_x = x;

                    if y > 0 && y % 2 == 0 {
                        x += match rng.range(3) {
                            0 => -1,
                            1 => 0,
                            _ => 1,
                        };
                        x = x.clamp(0, width_i32.saturating_sub(1));
                    }

                    let symbol = if x < previous_x {
                        '/'
                    } else if x > previous_x {
                        '\\'
                    } else {
                        '|'
                    };

                    let coord = Coord::new(x, y);
                    let id = terminal.add_character(symbol, coord);

                    if let Some(character) = terminal.character_mut(id) {
                        let mut style = Style::default().with_foreground(LIGHTNING_COLOR);
                        style.bold = true;
                        character.set_appearance(symbol, style);
                        character.visible = true;
                    }

                    lightning_ids.push(id);

                    for (index, input_id) in input_ids.iter().enumerate() {
                        let is_hit = terminal
                            .character(*input_id)
                            .map(|character| character.position == coord)
                            .unwrap_or(false);

                        if is_hit {
                            glow[index] = 8;
                        }
                    }
                }

                flash_remaining = 2;
            }

            for cloud in &clouds {
                if let Some(character) = terminal.character_mut(cloud.id) {
                    let color = if flash_remaining > 0 {
                        Color::rgb(180, 190, 215)
                    } else {
                        CLOUD_COLOR
                    };
                    character.set_appearance(
                        cloud.symbol,
                        Style::default().with_foreground(color),
                    );
                }
            }

            for drop in &mut rain {
                drop.y = drop.y.saturating_add(drop.speed);

                if drop.y >= height_i32 {
                    drop.y = 0;
                    drop.x = rng.range(width) as i32;
                } else if storm_frame % 4 == 0 && rng.range(4) == 0 {
                    drop.x = (drop.x + 1).rem_euclid(width_i32.max(1));
                }

                if let Some(character) = terminal.character_mut(drop.id) {
                    character.visible = true;
                    character.set_position(Coord::new(drop.x, drop.y));
                    character.set_appearance(
                        drop.symbol,
                        Style::default().with_foreground(RAIN_COLOR),
                    );
                }
            }

            for (index, id) in input_ids.iter().enumerate() {
                if let Some(character) = terminal.character_mut(*id) {
                    let style = storm_text_style(glow[index], flash_remaining);
                    character.set_appearance(character.input_symbol, style);
                }
            }

            frames.push(terminal.render_frame());

            for value in &mut glow {
                *value = value.saturating_sub(1);
            }
            flash_remaining = flash_remaining.saturating_sub(1);
        }

        hide_characters(&mut terminal, &lightning_ids);

        for frame in 0..DISSIPATION_FRAMES {
            let hidden_rain = rain.len().saturating_mul(frame + 1) / DISSIPATION_FRAMES;

            for (index, drop) in rain.iter_mut().enumerate() {
                drop.y = drop.y.saturating_add(drop.speed);

                if drop.y >= height_i32 {
                    drop.y = 0;
                    drop.x = rng.range(width) as i32;
                }

                if let Some(character) = terminal.character_mut(drop.id) {
                    character.visible = index >= hidden_rain;
                    character.set_position(Coord::new(drop.x, drop.y));
                }
            }

            for cloud in &clouds {
                let distance = (frame + 1) as i32;
                let x = if cloud.starts_left {
                    cloud.target_x - distance
                } else {
                    cloud.target_x + distance
                };

                if let Some(character) = terminal.character_mut(cloud.id) {
                    character.set_position(Coord::new(x, 0));
                }
            }

            for (index, id) in input_ids.iter().enumerate() {
                if let Some(character) = terminal.character_mut(*id) {
                    character.set_appearance(
                        character.input_symbol,
                        storm_text_style(glow[index], 0),
                    );
                }
                glow[index] = glow[index].saturating_sub(1);
            }

            frames.push(terminal.render_frame());
        }

        let cloud_ids: Vec<CharacterId> = clouds.iter().map(|cloud| cloud.id).collect();
        let rain_ids: Vec<CharacterId> = rain.iter().map(|drop| drop.id).collect();
        hide_characters(&mut terminal, &cloud_ids);
        hide_characters(&mut terminal, &rain_ids);

        for frame in 0..FADE_FRAMES {
            let progress = (frame + 1) as f64 / FADE_FRAMES as f64;
            let color = mix_color((42, 45, 64), (210, 218, 235), progress);

            for id in &input_ids {
                if let Some(character) = terminal.character_mut(*id) {
                    character.visible = true;
                    character.set_appearance(
                        character.input_symbol,
                        Style::default().with_foreground(color),
                    );
                }
            }

            frames.push(terminal.render_frame());
        }

        for id in &input_ids {
            if let Some(character) = terminal.character_mut(*id) {
                character.visible = true;
                character.set_appearance(character.input_symbol, Style::default());
            }
        }

        frames.push(terminal.render_frame());
        frames
    }
}

struct Cloud {
    id: CharacterId,
    target_x: i32,
    starts_left: bool,
    symbol: char,
}

struct RainDrop {
    id: CharacterId,
    x: i32,
    y: i32,
    speed: i32,
    symbol: char,
}

fn storm_text_style(glow: u8, flash: u8) -> Style {
    if flash > 0 {
        let mut style = Style::default()
            .with_foreground(Color::rgb(230, 238, 255))
            .with_background(Color::rgb(65, 75, 110));
        style.bold = true;
        return style;
    }

    if glow > 0 {
        let intensity = u16::from(glow);
        let red = (75 + intensity * 10).min(255) as u8;
        let green = (100 + intensity * 12).min(255) as u8;
        let blue = (145 + intensity * 13).min(255) as u8;
        return Style::default().with_foreground(Color::rgb(red, green, blue));
    }

    Style::default().with_foreground(DARK_TEXT)
}

fn hide_characters(terminal: &mut Terminal, ids: &[CharacterId]) {
    for id in ids {
        if let Some(character) = terminal.character_mut(*id) {
            character.visible = false;
        }
    }
}

fn mix_color(start: (u8, u8, u8), end: (u8, u8, u8), progress: f64) -> Color {
    let progress = progress.clamp(0.0, 1.0);
    let channel = |from: u8, to: u8| {
        (f64::from(from) + (f64::from(to) - f64::from(from)) * progress)
            .round()
            .clamp(0.0, 255.0) as u8
    };

    Color::rgb(
        channel(start.0, end.0),
        channel(start.1, end.1),
        channel(start.2, end.2),
    )
}

fn seed_from_input(input: &str) -> u64 {
    let mut seed = 0xcbf2_9ce4_8422_2325_u64;

    for byte in input.bytes() {
        seed ^= u64::from(byte);
        seed = seed.wrapping_mul(0x0000_0100_0000_01b3);
    }

    seed ^ 0x7468_756e_6465_7273
}

struct StormRng {
    state: u64,
}

impl StormRng {
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
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state
    }

    fn range(&mut self, upper: usize) -> usize {
        if upper <= 1 {
            0
        } else {
            (self.next_u64() % upper as u64) as usize
        }
    }
}
