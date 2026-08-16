use super::Effect;
use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::easing::ease_out_quad;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair};

const LAUNCH_DURATION: usize = 22;
const EXPLOSION_DURATION: usize = 26;
const PARTICLE_COUNT: usize = 24;
const MAX_ACTIVITY_FRAMES: usize = 200;

static PARTICLE_COLORS: [Color; 6] = [
    Color::RED,
    Color::YELLOW,
    Color::MAGENTA,
    Color::CYAN,
    Color::GREEN,
    Color::new(255, 128, 0),
];

static PARTICLE_SYMBOLS: [&str; 6] = ["*", "●", "◦", "·", "+", "◆"];

pub struct Fireworks;

impl Fireworks {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Fireworks {
    fn name(&self) -> &str {
        "fireworks"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        firework_frames(input)
    }
}

#[derive(Clone, Copy)]
struct Shell {
    target: Coord,
}

fn firework_frames(input: &str) -> Vec<String> {
    let (width, height) = Terminal::autodetect_size();
    let mut terminal = Terminal::from_input(input, width.max(1), height.max(1));
    terminal.clear_canvas();

    let shells: Vec<Shell> = terminal
        .characters
        .iter()
        .map(|character| Shell {
            target: character.position,
        })
        .collect();

    let char_count = terminal.characters.len().max(1);
    let launch_delay = calculate_launch_delay(char_count);
    let activity_frames = char_count * launch_delay + LAUNCH_DURATION + EXPLOSION_DURATION;
    let frame_count = activity_frames.min(MAX_ACTIVITY_FRAMES).max(20);

    let mut frames = Vec::with_capacity(frame_count + 1);

    for frame_idx in 0..frame_count {
        terminal.clear_canvas();
        for (shell_idx, shell) in shells.iter().enumerate() {
            render_shell(&mut terminal, shell_idx, *shell, frame_idx, launch_delay);
        }
        frames.push(terminal.write_frame());
    }

    frames.push(draw_input_text(&mut terminal));
    frames
}

fn calculate_launch_delay(char_count: usize) -> usize {
    let overhead = LAUNCH_DURATION + EXPLOSION_DURATION;
    let delay = MAX_ACTIVITY_FRAMES.saturating_sub(overhead) / char_count;
    delay.clamp(1, 12)
}

fn render_shell(
    terminal: &mut Terminal,
    shell_idx: usize,
    shell: Shell,
    frame_idx: usize,
    launch_delay: usize,
) {
    let start_frame = shell_idx * launch_delay;
    let detonation_frame = start_frame + LAUNCH_DURATION;
    let final_frame = detonation_frame + EXPLOSION_DURATION;

    if frame_idx >= start_frame && frame_idx < detonation_frame {
        let t = (frame_idx - start_frame) as f32 / LAUNCH_DURATION as f32;
        let eased_t = ease_out_quad(t);
        let start = Coord::new(
            terminal.canvas.width as f32 / 2.0,
            terminal.canvas.height as f32 - 1.0,
        );
        let position = start.lerp(shell.target, eased_t);
        let symbol = if frame_idx % 2 == 0 { "●" } else { "○" };

        draw_cell(
            terminal,
            position,
            symbol,
            CellStyle::with_color_pair(ColorPair::new(Color::WHITE, Color::BLACK)),
        );
    } else if frame_idx >= detonation_frame && frame_idx < final_frame {
        let elapsed = (frame_idx - detonation_frame) as f32;

        for particle_idx in 0..PARTICLE_COUNT {
            let angle = ((shell_idx * 13 + particle_idx) % PARTICLE_COUNT) as f32
                * 2.0
                * std::f32::consts::PI
                / PARTICLE_COUNT as f32;
            let speed = 1.25 + ((shell_idx + particle_idx) % 6) as f32 * 0.18;
            let velocity_x = angle.cos() * speed;
            let velocity_y = angle.sin() * speed;
            let gravity = 0.035;

            let x = shell.target.x + velocity_x * elapsed;
            let y = shell.target.y + velocity_y * elapsed
                + 0.5 * gravity * elapsed * elapsed;

            let symbol =
                PARTICLE_SYMBOLS[(shell_idx * 5 + particle_idx) % PARTICLE_SYMBOLS.len()];
            let color = PARTICLE_COLORS[(shell_idx * 7 + particle_idx) % PARTICLE_COLORS.len()];

            draw_cell(
                terminal,
                Coord::new(x, y),
                symbol,
                CellStyle::with_color_pair(ColorPair::new(color, Color::BLACK)),
            );
        }
    }
}

fn draw_cell(terminal: &mut Terminal, position: Coord, symbol: &str, style: CellStyle) {
    let x = position.x.round() as i32;
    let y = position.y.round() as i32;

    if x >= 0 && y >= 0 && x < terminal.canvas.width as i32 && y < terminal.canvas.height as i32 {
        terminal.canvas.set_cell(x as u16, y as u16, Cell::new(symbol, style));
    }
}

fn draw_input_text(terminal: &mut Terminal) -> String {
    terminal.clear_canvas();

    let style = terminal.config.default_style;
    let characters: Vec<(u16, u16, String)> = terminal
        .characters
        .iter()
        .map(|character| {
            (
                character.position.x.round() as u16,
                character.position.y.round() as u16,
                character.input_symbol.clone(),
            )
        })
        .collect();

    for (x, y, symbol) in characters {
        if x < terminal.canvas.width && y < terminal.canvas.height {
            terminal.canvas.set_cell(x, y, Cell::new(symbol, style));
        }
    }

    terminal.write_frame()
}
