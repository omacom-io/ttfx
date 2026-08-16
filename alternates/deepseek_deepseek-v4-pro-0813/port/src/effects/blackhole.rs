
use super::Effect;
use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair};

pub struct Blackhole;

impl Blackhole {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Blackhole {
    fn name(&self) -> &str {
        "blackhole"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (width, height) = Terminal::autodetect_size();
        let mut terminal = Terminal::from_input(input, width, height);

        if terminal.characters.is_empty() {
            return vec![terminal.write_frame()];
        }

        let mut frames: Vec<String> = Vec::new();
        frames.push(terminal.write_frame());

        let center = Coord::new(width as f32 / 2.0, height as f32 / 2.0);
        let radius = (width as f32 * 0.3).round()
            .min((height as f32 * 0.2).round())
            .max(3.0);

        let char_count = terminal.characters.len();
        let blackhole_count = (char_count / 10).max(1).min(char_count);
        let blackhole_indices: Vec<usize> = (0..blackhole_count).collect();

        let mut ring_index: Vec<Option<usize>> = vec![None; char_count];
        for (bi, &idx) in blackhole_indices.iter().enumerate() {
            ring_index[idx] = Some(bi);
        }

        let ring_positions = ring_positions(center, radius, blackhole_count);

        let mut rng = Rng::new(0x1234_5678);

        let star_colors = [
            Color::WHITE,
            Color::new(160, 160, 160),
            Color::new(100, 100, 100),
        ];
        let star_symbols = ["*", ".", ":", "·"];
        let unstable_symbols = ["◦", "◎", "◉", "●", "◉", "◎", "◦"];
        let blackhole_color = Color::WHITE;

        for (i, ch) in terminal.characters.iter_mut().enumerate() {
            if blackhole_indices.contains(&i) {
                ch.output_symbol = "*".to_string();
                ch.style = CellStyle::with_color_pair(ColorPair::new(blackhole_color, Color::BLACK));
            } else {
                let sym = star_symbols[rng.usize_lt(star_symbols.len())];
                let col = star_colors[rng.usize_lt(star_colors.len())];
                ch.output_symbol = sym.to_string();
                ch.style = CellStyle::with_color_pair(ColorPair::new(col, Color::BLACK));
            }
        }

        let initial_input_positions: Vec<Coord> =
            terminal.characters.iter().map(|c| c.position).collect();

        let mut positions = initial_input_positions.clone();
        let mut star_initial_positions = vec![Coord::zero(); char_count];

        for i in 0..char_count {
            if !blackhole_indices.contains(&i) {
                star_initial_positions[i] = random_coord(&mut rng, width, height);
                positions[i] = star_initial_positions[i];
            }
        }

        let attract_frames = 50;
        let spin_frames = 40;
        let collapse_frames = 25;
        let total_frames = attract_frames + spin_frames + collapse_frames;

        for step in 1..=total_frames {
            if step <= attract_frames {
                let t = step as f32 / attract_frames as f32;

                for i in 0..char_count {
                    if let Some(bi) = ring_index[i] {
                        positions[i] = initial_input_positions[i]
                            .lerp(ring_positions[bi], easing::ease_in_out_sine(t));
                    } else {
                        positions[i] =
                            star_initial_positions[i].lerp(center, easing::ease_in_expo(t));
                    }
                }

                update_characters(&mut terminal, &positions);
                for ch in &mut terminal.characters {
                    ch.visible = true;
                }
            } else if step <= attract_frames + spin_frames {
                let spin_t = (step - attract_frames) as f32 / spin_frames as f32;

                for i in 0..char_count {
                    if let Some(bi) = ring_index[i] {
                        let base_angle = bi as f32 * std::f32::consts::TAU / blackhole_count as f32;
                        let angle = base_angle + spin_t * std::f32::consts::TAU * 0.5;
                        positions[i] = Coord::new(
                            center.x + radius * angle.cos(),
                            center.y + radius * angle.sin(),
                        );
                    }
                }

                update_characters(&mut terminal, &positions);
                for i in 0..char_count {
                    terminal.characters[i].visible = ring_index[i].is_some();
                }
            } else {
                let collapse_t =
                    (step - attract_frames - spin_frames) as f32 / collapse_frames as f32;

                for i in 0..char_count {
                    if ring_index[i].is_some() {
                        positions[i] = positions[i].lerp(center, easing::ease_in_expo(collapse_t));
                    }
                }

                update_characters(&mut terminal, &positions);

                for i in 0..char_count {
                    if let Some(bi) = ring_index[i] {
                        let sym_index = (bi + step) % unstable_symbols.len();
                        terminal.characters[i].output_symbol = unstable_symbols[sym_index].to_string();
                        terminal.characters[i].visible = true;
                    } else {
                        terminal.characters[i].visible = false;
                    }
                }
            }

            render_terminal(&mut terminal);
            frames.push(terminal.write_frame());
        }

        for ch in &mut terminal.characters {
            ch.visible = false;
        }
        terminal.canvas.clear();
        frames.push(terminal.write_frame());

        frames
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let old = self.0;
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        old
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }

    fn usize_lt(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

fn ring_positions(center: Coord, radius: f32, count: usize) -> Vec<Coord> {
    (0..count)
        .map(|i| {
            let angle = i as f32 * std::f32::consts::TAU / count as f32;
            Coord::new(center.x + radius * angle.cos(), center.y + radius * angle.sin())
        })
        .collect()
}

fn random_coord(rng: &mut Rng, width: u16, height: u16) -> Coord {
    let x = rng.range_f32(0.0, width.saturating_sub(1).max(1) as f32);
    let y = rng.range_f32(0.0, height.saturating_sub(1).max(1) as f32);
    Coord::new(x, y)
}

fn update_characters(terminal: &mut Terminal, positions: &[Coord]) {
    for (i, ch) in terminal.characters.iter_mut().enumerate() {
        ch.position = positions[i];
    }
}

fn render_terminal(terminal: &mut Terminal) {
    let chars = &terminal.characters;
    let canvas = &mut terminal.canvas;

    canvas.clear();

    for ch in chars {
        if !ch.visible {
            continue;
        }

        let x = ch.position.x.round();
        let y = ch.position.y.round();

        if x < 0.0 || y < 0.0 || x >= canvas.width as f32 || y >= canvas.height as f32 {
            continue;
        }

        canvas.set_cell(
            x as u16,
            y as u16,
            Cell::new(ch.output_symbol.clone(), ch.style),
        );
    }
}
