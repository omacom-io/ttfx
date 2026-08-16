use super::Effect;
use crate::engine::canvas::CellStyle;
use crate::engine::terminal::Terminal;
use crate::utils::graphics::Color;

pub struct Thunderstorm {
    // The effect is stateless; all simulation state lives in `frames()`.
}

impl Thunderstorm {
    pub fn new() -> Self {
        Self {}
    }
}

impl Effect for Thunderstorm {
    fn name(&self) -> &str {
        "thunderstorm"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        thunderstorm_frames(input)
    }
}

fn thunderstorm_frames(input: &str) -> Vec<String> {
    let (width, height) = Terminal::autodetect_size();
    let mut terminal = Terminal::from_input(input, width, height);

    // Snapshot the input text positions before we start clearing the canvas every frame.
    let base_chars: Vec<(u16, u16, String)> = terminal
        .characters
        .iter()
        .map(|c| {
            (
                c.position.x as u16,
                c.position.y as u16,
                c.input_symbol.clone(),
            )
        })
        .collect();

    // Small deterministic RNG so the effect does not depend on external crates.
    let mut rng = Rng::new(0x9E37_79B9_7F4A_7C15);
    let mut frames = Vec::new();
    let total_frames = 120u32;

    let drop_count = (width as usize * height as usize / 12).clamp(24, 320);
    let mut drops: Vec<RainDrop> = (0..drop_count)
        .map(|_| RainDrop::random(&mut rng, width, height))
        .collect();

    let mut flash_active = 0u8;
    let mut bolt: Vec<(u16, u16)> = Vec::new();

    for _ in 0..total_frames {
        // Update lightning state.
        if flash_active > 0 {
            flash_active -= 1;
        } else if rng.chance(0.06) {
            flash_active = (2 + rng.gen_range(0, 3)) as u8;
            bolt = generate_bolt(&mut rng, width, height);
        }

        let is_flashing = flash_active > 0;

        // Start each frame from a clean black canvas.
        terminal.canvas.clear();

        // Draw the original input text. During a flash, the text is bright and
        // takes on a slightly blue background for a harsh, electric look.
        for (x, y, symbol) in &base_chars {
            if let Some(cell) = terminal.canvas.get_mut(*x, *y) {
                cell.symbol = symbol.clone();
                cell.style = if is_flashing {
                    CellStyle::new(Color::new(240, 245, 255), Color::new(20, 20, 45))
                } else {
                    CellStyle::new(Color::new(80, 125, 210), Color::BLACK)
                };
            }
        }

        // Draw the active lightning bolt.
        if is_flashing {
            for (x, y) in &bolt {
                if let Some(cell) = terminal.canvas.get_mut(*x, *y) {
                    cell.symbol = "│".to_string();
                    cell.style = CellStyle::new(Color::new(200, 230, 255), Color::new(35, 35, 70));
                }
            }
        }

        // Draw rain above/over the text.
        for drop in &mut drops {
            drop.update(&mut rng, width, height);

            let style = CellStyle::new(Color::new(130, 180, 255), Color::BLACK);
            if let Some(cell) = terminal.canvas.get_mut(drop.x, drop.y) {
                cell.symbol = "│".to_string();
                cell.style = style;
            }

            for offset in 1..=drop.trail_len {
                let trail_y = drop.y as i32 - offset as i32;
                if trail_y >= 0 {
                    if let Some(cell) = terminal.canvas.get_mut(drop.x, trail_y as u16) {
                        cell.symbol = "·".to_string();
                        cell.style = CellStyle::new(Color::new(65, 105, 190), Color::BLACK);
                    }
                }
            }
        }

        frames.push(terminal.write_frame());
    }

    frames
}

fn generate_bolt(rng: &mut Rng, width: u16, height: u16) -> Vec<(u16, u16)> {
    let width_i32 = width.saturating_sub(1) as i32;
    let mut points = Vec::new();
    let mut x = rng.gen_range(0, width.max(1) as u32) as i32;

    for y in 0..height.max(1) {
        points.push((x.clamp(0, width_i32) as u16, y));
        if rng.chance(0.3) {
            x += rng.gen_range(0, 3) as i32 - 1;
        }
    }

    points
}

struct RainDrop {
    x: u16,
    y: u16,
    speed: u16,
    trail_len: u16,
}

impl RainDrop {
    fn random(rng: &mut Rng, width: u16, height: u16) -> Self {
        Self {
            x: rng.gen_range(0, width.max(1) as u32) as u16,
            y: rng.gen_range(0, height.max(1) as u32) as u16,
            speed: rng.gen_range(1, 4) as u16,
            trail_len: rng.gen_range(1, 4) as u16,
        }
    }

    fn update(&mut self, rng: &mut Rng, width: u16, height: u16) {
        self.y = self.y.saturating_add(self.speed);
        if self.y >= height {
            self.y = 0;
            self.x = rng.gen_range(0, width.max(1) as u32) as u16;
            self.speed = rng.gen_range(1, 4) as u16;
            self.trail_len = rng.gen_range(1, 4) as u16;
        }
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn gen_range(&mut self, low: u32, high: u32) -> u32 {
        if high <= low {
            return low;
        }
        low + self.next_u32() % (high - low)
    }

    fn chance(&mut self, probability: f32) -> bool {
        (self.gen_range(0, 10_000) as f32 / 10_000.0) < probability
    }
}
