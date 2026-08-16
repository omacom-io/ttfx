use super::Effect;
use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::geometry::Coord;
use crate::utils::graphics::Color;

pub struct Spotlights;

impl Spotlights {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Spotlights {
    fn name(&self) -> &str {
        "spotlights"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (width, height) = input_dimensions(input);
        let width = width.max(1);
        let height = height.max(1);

        let mut terminal = Terminal::from_input(input, width, height);

        let mut rng = Rng::new(seed());
        let spotlight_count = 6usize;
        let beam_radius = 6.5f32;
        let frame_count = 110usize;

        let mut positions = Vec::with_capacity(spotlight_count);
        let mut targets = Vec::with_capacity(spotlight_count);

        for _ in 0..spotlight_count {
            positions.push(random_coord(&mut rng, width, height));
            targets.push(random_coord(&mut rng, width, height));
        }

        let mut frames = Vec::with_capacity(frame_count + 1);

        for _ in 0..frame_count {
            for i in 0..spotlight_count {
                let speed = 0.012 + (i as f32 * 0.006);
                positions[i] = positions[i].lerp(targets[i], speed);

                if positions[i].distance(targets[i]) < 1.0 {
                    targets[i] = random_coord(&mut rng, width, height);
                }
            }

            render_spotlight_frame(&mut terminal, &positions, beam_radius);
            frames.push(terminal.write_frame());
        }

        let characters = terminal.characters.clone();
        terminal.clear_canvas();
        for character in &characters {
            let x = character.position.x.round() as u16;
            let y = character.position.y.round() as u16;
            let x = x.min(width - 1);
            let y = y.min(height - 1);

            terminal.canvas.set_cell(
                x,
                y,
                Cell::new(character.input_symbol.clone(), CellStyle::default()),
            );
        }
        frames.push(terminal.write_frame());

        frames
    }
}

fn render_spotlight_frame(terminal: &mut Terminal, positions: &[Coord], beam_radius: f32) {
    let characters = terminal.characters.clone();
    terminal.clear_canvas();

    for character in &characters {
        let pos = character.position;

        let nearest = positions
            .iter()
            .map(|p| p.distance(pos))
            .fold(f32::MAX, f32::min);

        let falloff = (nearest / beam_radius).clamp(0.0, 1.0);
        let intensity = (1.0 - falloff) * (1.0 - falloff);

        let r = (255.0 * intensity) as u8;
        let g = (220.0 * intensity) as u8;
        let b = (160.0 * intensity) as u8;

        let mut style = CellStyle::new(Color::new(r, g, b), Color::BLACK);
        if intensity < 0.06 {
            style.hidden = true;
        }

        let x = pos.x.round() as u16;
        let y = pos.y.round() as u16;
        let x = x.min(terminal.canvas.width - 1);
        let y = y.min(terminal.canvas.height - 1);

        terminal.canvas.set_cell(x, y, Cell::new(character.input_symbol.clone(), style));
    }
}

fn input_dimensions(input: &str) -> (u16, u16) {
    let mut max_width: u16 = 0;
    let mut current_width: u16 = 0;
    let mut height: u16 = 1;

    for ch in input.chars() {
        if ch == '\n' {
            max_width = max_width.max(current_width);
            height += 1;
            current_width = 0;
        } else {
            current_width += 1;
        }
    }

    max_width = max_width.max(current_width);
    (max_width.max(1), height.max(1))
}

fn random_coord(rng: &mut Rng, width: u16, height: u16) -> Coord {
    let x = rng.next_range(0.0, width as f32 - 1.0);
    let y = rng.next_range(0.0, height as f32 - 1.0);
    Coord::new(x, y)
}

fn seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x9E3779B97F4A7C15 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u64() & 0x00FFFFFF) as f32 / 0x00FFFFFF as f32
    }

    fn next_range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_f32()
    }
}
