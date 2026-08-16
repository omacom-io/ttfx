use super::Effect;
use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::Color;

const FRAME_COUNT: usize = 80;
const PALETTE: [Color; 7] = [
    Color::RED,
    Color::GREEN,
    Color::BLUE,
    Color::CYAN,
    Color::MAGENTA,
    Color::YELLOW,
    Color::WHITE,
];

pub struct Bouncyballs {
    frame_count: usize,
}

impl Bouncyballs {
    pub fn new() -> Self {
        Self {
            frame_count: FRAME_COUNT,
        }
    }
}

impl Effect for Bouncyballs {
    fn name(&self) -> &str {
        "bouncyballs"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (width, height) = input_dimensions(input);
        let mut terminal = Terminal::from_input(input, width, height);

        let characters: Vec<(String, Coord)> = terminal
            .characters
            .iter()
            .map(|c| (c.input_symbol.clone(), c.position))
            .collect();

        let mut rng = Rng::new(0x5EED_1234_5678_9ABC);
        let mut balls: Vec<BallAnim> = characters
            .into_iter()
            .map(|(symbol, target)| BallAnim::new(symbol, target, &mut rng, width, height, &PALETTE))
            .collect();

        if balls.is_empty() {
            balls.push(BallAnim::new(
                "*".to_string(),
                Coord::new(0.0, 0.0),
                &mut rng,
                width,
                height,
                &PALETTE,
            ));
        }

        let mut frames = Vec::with_capacity(self.frame_count);

        for frame_index in 0..self.frame_count {
            terminal.clear_canvas();

            let t = frame_index as f32 / (self.frame_count - 1).max(1) as f32;
            let eased = easing::ease_out_bounce(t);

            for ball in &balls {
                let pos = ball.start.lerp(ball.target, eased);
                let x = pos.x.round() as u16;
                let y = pos.y.round() as u16;

                if x < terminal.canvas.width && y < terminal.canvas.height {
                    let style = CellStyle::new(ball.color, Color::BLACK);
                    terminal.canvas.set_cell(x, y, Cell::new(ball.symbol.clone(), style));
                }
            }

            frames.push(terminal.write_frame());
        }

        frames
    }
}

struct BallAnim {
    symbol: String,
    start: Coord,
    target: Coord,
    color: Color,
}

impl BallAnim {
    fn new(
        symbol: String,
        target: Coord,
        rng: &mut Rng,
        width: u16,
        height: u16,
        palette: &[Color],
    ) -> Self {
        let max_x = width.saturating_sub(1).max(1) as f32;
        let max_y = height.saturating_sub(1).max(1) as f32;

        let start_x = rng.next_range(0.0, max_x);
        let start_y = rng.next_range(0.0, max_y);
        let color = palette[rng.next_u64() as usize % palette.len()];

        Self {
            symbol,
            start: Coord::new(start_x, start_y),
            target,
            color,
        }
    }
}

fn input_dimensions(input: &str) -> (u16, u16) {
    let normalized = input.replace('\r', "");
    let lines: Vec<&str> = normalized.split('\n').collect();

    let width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
        .clamp(10, 120) as u16;

    let height = lines.len().clamp(10, 60) as u16;

    (width, height)
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u64() as f32) / (u64::MAX as f32)
    }

    fn next_range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_f32()
    }
}
