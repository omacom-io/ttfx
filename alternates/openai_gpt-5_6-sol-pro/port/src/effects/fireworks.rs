
use std::f64::consts::TAU;

use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::Terminal;
use crate::utils::easing::{in_out_sine, out_expo, out_quad};
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Style};

pub struct Fireworks;

impl Fireworks {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Fireworks {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
struct Particle {
    id: CharacterId,
    symbol: char,
    home: Coord,
    launch: Coord,
    apex: Coord,
    burst: Coord,
    start_tick: usize,
    shell_color: Color,
}

impl Effect for Fireworks {
    fn name(&self) -> &str {
        "fireworks"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        const LAUNCH_STEPS: usize = 14;
        const BURST_STEPS: usize = 9;
        const RETURN_STEPS: usize = 22;
        const LAUNCH_DELAY: usize = 7;

        let mut terminal = Terminal::from_text(input);
        let width = terminal.canvas().width();
        let height = terminal.canvas().height();

        let characters = terminal
            .characters()
            .iter()
            .filter(|character| !character.input_symbol.is_whitespace())
            .map(|character| {
                (
                    character.id,
                    character.input_symbol,
                    character.position,
                )
            })
            .collect::<Vec<_>>();

        if characters.is_empty() {
            return vec![terminal.render_frame()];
        }

        let shell_size = characters.len().saturating_add(49) / 50;
        let shell_size = shell_size.max(1);
        let mut particles = Vec::with_capacity(characters.len());

        for (shell_index, shell) in characters.chunks(shell_size).enumerate() {
            let shell_seed = mix_seed(shell_index as u64 + 1);
            let apex_x = (shell_seed as usize % width) as i32;

            let apex_region = ((height as f64 * 0.65).ceil() as usize)
                .max(1)
                .min(height);
            let apex_y = (mix_seed(shell_seed) as usize % apex_region) as i32;
            let launch = Coord::new(apex_x, height.saturating_sub(1) as i32);
            let apex = Coord::new(apex_x, apex_y);
            let shell_color = FIREWORK_COLORS[shell_index % FIREWORK_COLORS.len()];
            let phase = unit_value(mix_seed(shell_seed ^ 0xa5a5_a5a5)) * TAU;

            let base_radius = ((width.max(height) as f64) * 0.12).round().max(1.0);

            for (particle_index, &(id, symbol, home)) in shell.iter().enumerate() {
                let angle = phase + TAU * particle_index as f64 / shell.len() as f64;
                let radius_seed = mix_seed(shell_seed ^ particle_index as u64);
                let radius = base_radius * (0.7 + unit_value(radius_seed) * 0.6);

                let burst_x = apex.x + (angle.cos() * radius).round() as i32;
                let burst_y = apex.y + (angle.sin() * radius * 0.55).round() as i32;
                let burst = Coord::new(
                    burst_x.clamp(0, width.saturating_sub(1) as i32),
                    burst_y.clamp(0, height.saturating_sub(1) as i32),
                );

                particles.push(Particle {
                    id,
                    symbol,
                    home,
                    launch,
                    apex,
                    burst,
                    start_tick: shell_index * LAUNCH_DELAY,
                    shell_color,
                });
            }
        }

        for particle in &particles {
            if let Some(character) = terminal.character_mut(particle.id) {
                character.visible = false;
            }
        }

        let last_start = particles
            .iter()
            .map(|particle| particle.start_tick)
            .max()
            .unwrap_or(0);
        let final_tick = last_start + LAUNCH_STEPS + BURST_STEPS + RETURN_STEPS;
        let mut frames = Vec::with_capacity(final_tick + 1);

        for tick in 0..=final_tick {
            for particle in &particles {
                let Some(character) = terminal.character_mut(particle.id) else {
                    continue;
                };

                if tick < particle.start_tick {
                    character.visible = false;
                    continue;
                }

                let local_tick = tick - particle.start_tick;
                character.visible = true;

                if local_tick < LAUNCH_STEPS {
                    let progress = local_tick as f64 / LAUNCH_STEPS as f64;
                    let position = particle.launch.lerp(particle.apex, out_quad(progress));
                    let launch_symbol = launch_symbol(progress);

                    character.set_position(position);
                    character.set_appearance(
                        launch_symbol,
                        Style::default().with_foreground(particle.shell_color),
                    );
                } else if local_tick < LAUNCH_STEPS + BURST_STEPS {
                    let burst_tick = local_tick - LAUNCH_STEPS;
                    let progress = burst_tick as f64 / BURST_STEPS as f64;
                    let position = particle.apex.lerp(particle.burst, out_expo(progress));

                    character.set_position(position);
                    character.set_appearance(
                        particle.symbol,
                        Style::default().with_foreground(particle.shell_color),
                    );
                } else if local_tick < LAUNCH_STEPS + BURST_STEPS + RETURN_STEPS {
                    let return_tick = local_tick - LAUNCH_STEPS - BURST_STEPS;
                    let progress = return_tick as f64 / RETURN_STEPS as f64;
                    let position = particle
                        .burst
                        .lerp(particle.home, in_out_sine(progress));
                    let color = blend_color(
                        particle.shell_color,
                        final_color(particle.home, height),
                        progress,
                    );

                    character.set_position(position);
                    character.set_appearance(
                        particle.symbol,
                        Style::default().with_foreground(color),
                    );
                } else {
                    character.set_position(particle.home);
                    character.set_appearance(
                        particle.symbol,
                        Style::default()
                            .with_foreground(final_color(particle.home, height)),
                    );
                }
            }

            frames.push(terminal.render_frame());
        }

        frames
    }
}

const FIREWORK_COLORS: [Color; 6] = [
    Color::rgb(136, 206, 235),
    Color::rgb(255, 255, 255),
    Color::rgb(254, 95, 85),
    Color::rgb(240, 182, 127),
    Color::rgb(214, 209, 177),
    Color::rgb(199, 239, 207),
];

fn launch_symbol(progress: f64) -> char {
    const SYMBOLS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let index = (progress.clamp(0.0, 1.0) * (SYMBOLS.len() - 1) as f64).round() as usize;
    SYMBOLS[index]
}

fn final_color(coord: Coord, height: usize) -> Color {
    let denominator = height.saturating_sub(1).max(1) as f64;
    let progress = (coord.y.max(0) as f64 / denominator).clamp(0.0, 1.0);

    if progress < 0.5 {
        blend_color(
            Color::rgb(138, 0, 138),
            Color::rgb(0, 209, 255),
            progress * 2.0,
        )
    } else {
        blend_color(
            Color::rgb(0, 209, 255),
            Color::rgb(255, 255, 255),
            (progress - 0.5) * 2.0,
        )
    }
}

fn blend_color(start: Color, end: Color, progress: f64) -> Color {
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
            blend_channel(start_r, end_r, progress),
            blend_channel(start_g, end_g, progress),
            blend_channel(start_b, end_b, progress),
        ),
        _ if progress < 0.5 => start,
        _ => end,
    }
}

fn blend_channel(start: u8, end: u8, progress: f64) -> u8 {
    (start as f64 + (end as f64 - start as f64) * progress)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn mix_seed(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn unit_value(value: u64) -> f64 {
    (value >> 11) as f64 / ((1_u64 << 53) - 1) as f64
}
