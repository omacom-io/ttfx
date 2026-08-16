//! vhstape: lines of text glitch horizontally like a worn VHS tape, the
//! glitching intensifies into full waves, the screen dissolves into static
//! snow, then the text is redrawn row by row in the final gradient.
//!
//! Port of terminaltexteffects/effects/effect_vhstape.py, adapted to the
//! simplified engine in this crate.

use super::Effect;
use crate::engine::motion::Motion;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

const GLITCH_COLORS: [&str; 5] = ["ffffff", "ff0000", "00ff00", "0000ff", "ffffff"];
const NOISE_COLORS: [&str; 6] = ["1e1e1f", "3c3b3d", "6d6c70", "a2a1a6", "cbc9cf", "ffffff"];
const FINAL_STOPS: [&str; 3] = ["ab48ff", "e7b2b2", "fffebd"];
const NOISE_SYMBOLS: [char; 4] = ['#', '*', '.', ':'];

/// Small deterministic xorshift PRNG so the effect is self-contained.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9e37_79b9_7f4a_7c15 } else { seed })
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Random usize in `[lo, hi)`.
    fn gen_range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() as usize) % (hi - lo)
    }

    /// True with probability `p`.
    fn chance(&mut self, p: f64) -> bool {
        ((self.next_u64() % 10_000) as f64 / 10_000.0) < p
    }
}

#[derive(Clone, Copy)]
enum LineState {
    Idle,
    Out { left: bool, ticks: u32 },
    Hold { left: bool, ticks: u32 },
    Back { ticks: u32 },
}

#[derive(Clone, Copy)]
enum WaveState {
    Waiting,
    Mid(u32),
    End(u32),
    Home(u32),
    Done,
}

fn add_path(motion: &mut Motion, id: &str, speed: f64, coords: &[Coord]) {
    let path = motion.new_path(id, speed, None);
    for c in coords {
        path.add_waypoint(*c);
    }
}

fn render(terminal: &mut Terminal, frames: &mut Vec<String>) {
    terminal.tick();
    frames.push(terminal.get_formatted_output_string());
}

fn advance_lines(
    states: &mut [LineState],
    rows: &[Vec<usize>],
    terminal: &mut Terminal,
    rng: &mut Rng,
) {
    for i in 0..states.len() {
        states[i] = match states[i] {
            LineState::Idle => LineState::Idle,
            LineState::Out { left, ticks } => {
                if ticks <= 1 {
                    LineState::Hold {
                        left,
                        ticks: 1 + (rng.next_u64() % 5) as u32,
                    }
                } else {
                    LineState::Out {
                        left,
                        ticks: ticks - 1,
                    }
                }
            }
            LineState::Hold { left, ticks } => {
                if ticks <= 1 {
                    for &ci in &rows[i] {
                        let ch = &mut terminal.characters[ci];
                        ch.motion.activate_path(if left {
                            "glitch_left_home"
                        } else {
                            "glitch_right_home"
                        });
                    }
                    LineState::Back { ticks: 3 }
                } else {
                    LineState::Hold {
                        left,
                        ticks: ticks - 1,
                    }
                }
            }
            LineState::Back { ticks } => {
                if ticks <= 1 {
                    for &ci in &rows[i] {
                        terminal.characters[ci].animation.activate_scene("base");
                    }
                    LineState::Idle
                } else {
                    LineState::Back { ticks: ticks - 1 }
                }
            }
        };
    }
}

fn advance_wave(states: &mut [WaveState], rows: &[Vec<usize>], terminal: &mut Terminal) {
    for i in 0..states.len() {
        states[i] = match states[i] {
            WaveState::Waiting => WaveState::Waiting,
            WaveState::Done => WaveState::Done,
            WaveState::Mid(t) => {
                if t <= 1 {
                    for &ci in &rows[i] {
                        terminal.characters[ci].motion.activate_path("wave_end");
                    }
                    WaveState::End(4)
                } else {
                    WaveState::Mid(t - 1)
                }
            }
            WaveState::End(t) => {
                if t <= 1 {
                    for &ci in &rows[i] {
                        terminal.characters[ci].motion.activate_path("wave_home");
                    }
                    WaveState::Home(3)
                } else {
                    WaveState::End(t - 1)
                }
            }
            WaveState::Home(t) => {
                if t <= 1 {
                    for &ci in &rows[i] {
                        terminal.characters[ci].animation.activate_scene("base");
                    }
                    WaveState::Done
                } else {
                    WaveState::Home(t - 1)
                }
            }
        };
    }
}

pub struct Vhstape;

impl Vhstape {
    pub fn new() -> Self {
        Vhstape
    }
}

impl Default for Vhstape {
    fn default() -> Self {
        Vhstape::new()
    }
}

impl Effect for Vhstape {
    fn name(&self) -> &str {
        "vhstape"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let mut rng = Rng::new(0x5eed_cafe_1234_5678);
        let mut frames: Vec<String> = Vec::new();

        let glitch_colors: Vec<Color> = GLITCH_COLORS
            .iter()
            .filter_map(|h| Color::from_hex(h))
            .collect();
        let noise_colors: Vec<Color> = NOISE_COLORS
            .iter()
            .filter_map(|h| Color::from_hex(h))
            .collect();
        let stops: Vec<Color> = FINAL_STOPS
            .iter()
            .filter_map(|h| Color::from_hex(h))
            .collect();
        let final_gradient = Gradient::new(&stops, 12);
        let height = terminal.canvas.height;
        let white = Color::new(255, 255, 255);

        // ---------- build: scenes and paths for every character ----------
        for i in 0..terminal.characters.len() {
            let (symbol, home) = {
                let ch = &terminal.characters[i];
                (ch.input_symbol, ch.input_coord)
            };
            let fraction = if height > 1 {
                (home.row - 1) as f64 / (height - 1) as f64
            } else {
                0.0
            };
            let final_color = final_gradient.get_color_at_fraction(fraction).unwrap_or(white);

            let ch = &mut terminal.characters[i];

            {
                let base = ch.animation.new_scene("base", false);
                base.add_frame(symbol, 1, ColorPair::fg(final_color), false);
            }
            {
                let glitch = ch.animation.new_scene("glitch", true);
                for c in &glitch_colors {
                    glitch.add_frame(symbol, 1, ColorPair::fg(*c), false);
                }
            }
            {
                let wave = ch.animation.new_scene("wave", true);
                for c in &glitch_colors {
                    wave.add_frame(symbol, 2, ColorPair::fg(*c), true);
                }
            }
            {
                let noise = ch.animation.new_scene("noise", true);
                for _ in 0..25 {
                    let s = NOISE_SYMBOLS[rng.gen_range(0, NOISE_SYMBOLS.len())];
                    let c = noise_colors[rng.gen_range(0, noise_colors.len())];
                    noise.add_frame(s, 2, ColorPair::fg(c), false);
                }
            }

            let right = Coord::new(home.column + 4, home.row);
            let left = Coord::new(home.column - 4, home.row);
            let mid = Coord::new(home.column + 8, home.row);
            let end = Coord::new(home.column + 14, home.row);
            add_path(&mut ch.motion, "glitch_right", 2.0, &[home, right]);
            add_path(&mut ch.motion, "glitch_right_home", 2.0, &[right, home]);
            add_path(&mut ch.motion, "glitch_left", 2.0, &[home, left]);
            add_path(&mut ch.motion, "glitch_left_home", 2.0, &[left, home]);
            add_path(&mut ch.motion, "wave_mid", 2.0, &[home, mid]);
            add_path(&mut ch.motion, "wave_end", 2.0, &[mid, end]);
            add_path(&mut ch.motion, "wave_home", 7.0, &[end, home]);

            ch.is_visible = true;
            ch.animation.activate_scene("base");
        }

        // Group character indices by row, top row first.
        let mut row_values: Vec<i32> = terminal
            .characters
            .iter()
            .map(|c| c.input_coord.row)
            .collect();
        row_values.sort_unstable();
        row_values.dedup();
        row_values.reverse();
        let rows: Vec<Vec<usize>> = row_values
            .iter()
            .map(|&r| {
                terminal
                    .characters
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.input_coord.row == r)
                    .map(|(i, _)| i)
                    .collect()
            })
            .collect();

        if rows.is_empty() {
            frames.push(terminal.get_formatted_output_string());
            return frames;
        }

        render(&mut terminal, &mut frames);

        // ---------- phase 1: random line glitches ----------
        let glitch_ticks = 150usize;
        let mut states: Vec<LineState> = vec![LineState::Idle; rows.len()];
        for _ in 0..glitch_ticks {
            if rng.chance(0.06) {
                let idx = rng.gen_range(0, rows.len());
                if matches!(states[idx], LineState::Idle) {
                    let left = rng.chance(0.5);
                    for &ci in &rows[idx] {
                        let ch = &mut terminal.characters[ci];
                        ch.animation.activate_scene("glitch");
                        ch.motion
                            .activate_path(if left { "glitch_left" } else { "glitch_right" });
                    }
                    states[idx] = LineState::Out { left, ticks: 3 };
                }
            }
            advance_lines(&mut states, &rows, &mut terminal, &mut rng);
            render(&mut terminal, &mut frames);
        }
        // Drain: let every glitching line return home.
        let mut guard = 0;
        while states.iter().any(|s| !matches!(s, LineState::Idle)) && guard < 200 {
            advance_lines(&mut states, &rows, &mut terminal, &mut rng);
            render(&mut terminal, &mut frames);
            guard += 1;
        }

        // ---------- phase 2: glitch waves sweeping the rows ----------
        for _wave in 0..2 {
            let mut wstates = vec![WaveState::Waiting; rows.len()];
            let mut front = 0usize;
            let mut ticks = 0usize;
            loop {
                if front < rows.len() && ticks % 2 == 0 {
                    for &ci in &rows[front] {
                        let ch = &mut terminal.characters[ci];
                        ch.animation.activate_scene("wave");
                        ch.motion.activate_path("wave_mid");
                    }
                    wstates[front] = WaveState::Mid(5);
                    front += 1;
                }
                advance_wave(&mut wstates, &rows, &mut terminal);
                render(&mut terminal, &mut frames);
                ticks += 1;
                if front >= rows.len()
                    && wstates.iter().all(|s| matches!(s, WaveState::Done))
                {
                    break;
                }
                if ticks > 2000 {
                    break;
                }
            }
        }

        // ---------- phase 3: dissolve into static snow ----------
        let char_count = terminal.characters.len();
        let mut snowing = vec![false; char_count];
        let mut ticks = 0usize;
        loop {
            for i in 0..char_count {
                if !snowing[i] && rng.chance(0.08) {
                    terminal.characters[i].animation.activate_scene("noise");
                    snowing[i] = true;
                }
            }
            render(&mut terminal, &mut frames);
            ticks += 1;
            if snowing.iter().all(|&s| s) || ticks > 120 {
                break;
            }
        }
        for i in 0..char_count {
            if !snowing[i] {
                terminal.characters[i].animation.activate_scene("noise");
            }
        }
        for _ in 0..40 {
            render(&mut terminal, &mut frames);
        }

        // ---------- phase 4: redraw rows bottom-to-top in the final gradient ----------
        for row in rows.iter().rev() {
            for &ci in row {
                let ch = &mut terminal.characters[ci];
                ch.motion.current_coord = ch.input_coord;
                ch.animation.activate_scene("base");
            }
            render(&mut terminal, &mut frames);
            render(&mut terminal, &mut frames);
        }

        // Hold the final image briefly.
        for _ in 0..10 {
            render(&mut terminal, &mut frames);
        }

        frames
    }
}
