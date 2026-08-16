use super::Effect;
use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, Gradient};

#[derive(Debug, Clone, Copy)]
pub struct Unstable;

impl Unstable {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Unstable {
    fn name(&self) -> &str {
        "unstable"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        unstable_frames(input)
    }
}

fn random_unit(seed: u32) -> f32 {
    let mut x = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    x ^= x >> 16;
    x = x.wrapping_mul(1103515245).wrapping_add(12345);
    (x >> 8) as f32 / 16_777_215.0
}

fn random_symbol(seed: u32) -> String {
    const SYMBOLS: &[&str] = &["*", "+", "o", "O", "0", ".", ":", "x"];
    let idx = (random_unit(seed.wrapping_mul(31)) * SYMBOLS.len() as f32) as usize;
    SYMBOLS[idx.min(SYMBOLS.len() - 1)].to_string()
}

fn explosion_target(seed: u32, origin: Coord, width: u16, height: u16) -> Coord {
    let angle = random_unit(seed.wrapping_add(1)) * 2.0 * std::f32::consts::PI;
    let max_dim = if width > height { width } else { height };
    let radius = 5.0 + random_unit(seed.wrapping_add(2)) * (max_dim as f32 * 0.6);

    Coord::new(
        origin.x + angle.cos() * radius,
        origin.y + angle.sin() * radius * 0.5,
    )
}

fn push_frame(
    terminal: &mut Terminal,
    positions: &[Coord],
    symbols: &[String],
    styles: &[CellStyle],
) {
    terminal.clear_canvas();

    for (i, &pos) in positions.iter().enumerate() {
        if pos.x < 0.0 || pos.y < 0.0 {
            continue;
        }

        let x = pos.x.round() as u16;
        let y = pos.y.round() as u16;

        if x >= terminal.canvas.width || y >= terminal.canvas.height {
            continue;
        }

        terminal
            .canvas
            .set_cell(x, y, Cell::new(symbols[i].clone(), styles[i]));
    }
}

fn unstable_frames(input: &str) -> Vec<String> {
    let (width, height) = Terminal::autodetect_size();
    let mut terminal = Terminal::from_input(input, width, height);

    if terminal.characters.is_empty() {
        return vec![terminal.write_frame()];
    }

    let count = terminal.characters.len();
    let origins: Vec<Coord> = terminal.characters.iter().map(|c| c.position).collect();
    let input_symbols: Vec<String> = terminal
        .characters
        .iter()
        .map(|c| c.input_symbol.clone())
        .collect();
    let targets: Vec<Coord> = origins
        .iter()
        .enumerate()
        .map(|(i, &origin)| explosion_target(i as u32, origin, width, height))
        .collect();

    let explosion_gradient = Gradient::new()
        .add_stop(0.0, Color::RED)
        .add_stop(0.6, Color::YELLOW)
        .add_stop(1.0, Color::WHITE);
    let reassembly_gradient = Gradient::new()
        .add_stop(0.0, Color::MAGENTA)
        .add_stop(0.6, Color::CYAN)
        .add_stop(1.0, Color::WHITE);

    let mut frames = Vec::new();

    // Rumble: short jitter before the text flies apart.
    for step in 0..10usize {
        let t = step as f32 / 10.0;
        let positions: Vec<Coord> = origins
            .iter()
            .enumerate()
            .map(|(i, &origin)| {
                let jitter_x = random_unit(i as u32 + 3 * step as u32) * 2.0 - 1.0;
                let jitter_y = random_unit(i as u32 + 7 * step as u32) * 2.0 - 1.0;
                Coord::new(origin.x + jitter_x, origin.y + jitter_y)
            })
            .collect();
        let symbols: Vec<String> = positions
            .iter()
            .enumerate()
            .map(|(i, _)| {
                if t < 0.5 && random_unit(i as u32 + 11 * step as u32) > 0.65 {
                    random_symbol(i as u32 + step as u32)
                } else {
                    input_symbols[i].clone()
                }
            })
            .collect();
        let styles = vec![CellStyle::new(Color::YELLOW, Color::BLACK); count];

        push_frame(&mut terminal, &positions, &symbols, &styles);
        frames.push(terminal.write_frame());
    }

    // Explosion: origin -> random scatter coordinates.
    for step in 0..35usize {
        let t = step as f32 / 35.0;
        let eased = easing::ease_out_cubic(t);
        let positions: Vec<Coord> = origins
            .iter()
            .zip(targets.iter())
            .map(|(&a, &b)| a.lerp(b, eased))
            .collect();
        let symbols: Vec<String> = positions
            .iter()
            .enumerate()
            .map(|(i, _)| {
                if random_unit(i as u32 + 13 * step as u32) > 0.7 {
                    random_symbol(i as u32 + step as u32)
                } else {
                    input_symbols[i].clone()
                }
            })
            .collect();
        let styles: Vec<CellStyle> = (0..count)
            .map(|_| CellStyle::new(explosion_gradient.color_at(eased), Color::BLACK))
            .collect();

        push_frame(&mut terminal, &positions, &symbols, &styles);
        frames.push(terminal.write_frame());
    }

    // Reassembly: scatter -> origin.
    for step in 0..45usize {
        let t = step as f32 / 45.0;
        let eased = easing::ease_in_cubic(t);
        let positions: Vec<Coord> = targets
            .iter()
            .zip(origins.iter())
            .map(|(&a, &b)| a.lerp(b, eased))
            .collect();
        let symbols: Vec<String> = positions
            .iter()
            .enumerate()
            .map(|(i, _)| {
                if eased < 0.7 && random_unit(i as u32 + 23 * step as u32) > 0.65 {
                    random_symbol(i as u32 + step as u32)
                } else {
                    input_symbols[i].clone()
                }
            })
            .collect();
        let styles: Vec<CellStyle> = (0..count)
            .map(|_| CellStyle::new(reassembly_gradient.color_at(eased), Color::BLACK))
            .collect();

        push_frame(&mut terminal, &positions, &symbols, &styles);
        frames.push(terminal.write_frame());
    }

    // Settle: original symbols with the terminal default style.
    let final_positions = origins.clone();
    let final_styles = vec![CellStyle::default(); count];
    for _ in 0..8 {
        push_frame(
            &mut terminal,
            &final_positions,
            &input_symbols,
            &final_styles,
        );
        frames.push(terminal.write_frame());
    }

    frames
}
