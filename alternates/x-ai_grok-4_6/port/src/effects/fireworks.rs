use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{
    find_coord_on_bezier_curve, find_length_of_bezier_curve, find_length_of_line, lerp_coord, Coord,
};
use crate::utils::graphics::{Color, ColorPair, Gradient};
use crate::utils::round_half_even;

const FIREWORK_SYMBOL: &str = "o";
const FIREWORK_VOLUME: f64 = 0.02;
const LAUNCH_DELAY: i32 = 60;
const EXPLODE_DISTANCE_PCT: i32 = 10;
const ORIGIN_SPEED: f64 = 0.2;
const EXPLODE_SPEED: f64 = 0.15;
const DROP_SPEED: f64 = 0.2;
const BLOSSOM_HOLD: usize = 2;

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

impl Effect for Fireworks {
    fn name(&self) -> &str {
        "fireworks"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        simulate(input)
    }
}

fn hex(s: &str) -> Color {
    Color::from_hex(s).unwrap_or(Color::rgb(255, 255, 255))
}

fn out_expo(t: f64) -> f64 {
    if t >= 1.0 {
        1.0
    } else {
        1.0 - 2.0_f64.powf(-10.0 * t)
    }
}

fn in_expo(t: f64) -> f64 {
    if t <= 0.0 {
        0.0
    } else {
        2.0_f64.powf(10.0 * t - 10.0)
    }
}

fn out_circ(t: f64) -> f64 {
    (1.0 - (t - 1.0) * (t - 1.0)).max(0.0).sqrt()
}

struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed | 1 }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }

    fn usize(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() as usize) % n
        }
    }

    fn i32(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            lo
        } else {
            lo + self.usize((hi - lo + 1) as usize) as i32
        }
    }
}

fn random_coord(rng: &mut Rng, left: i32, right: i32, bottom: i32, top: i32) -> Coord {
    let (c0, c1) = if left <= right {
        (left, right)
    } else {
        (right, left)
    };
    let (r0, r1) = if bottom <= top {
        (bottom, top)
    } else {
        (top, bottom)
    };
    Coord::new(rng.i32(c0, c1), rng.i32(r0, r1))
}

fn random_near(rng: &mut Rng, origin: Coord, max_dist: i32) -> Coord {
    if max_dist <= 0 {
        return origin;
    }
    let angle = rng.f64() * std::f64::consts::TAU;
    let dist = f64::from(rng.i32(1, max_dist));
    Coord {
        column: origin.column + round_half_even(dist * angle.cos()) as i32,
        row: origin.row + round_half_even(dist * angle.sin()) as i32,
    }
}

fn advance(progress: f64, speed: f64, length: f64) -> f64 {
    let len = if length < 1e-9 { 1.0 } else { length };
    (progress + speed / len).min(1.0)
}

fn blossom_palette(firework: Color) -> Vec<Color> {
    let white = Color::rgb(255, 255, 255);
    let up = Gradient::new(&[firework, white], 6);
    let down = Gradient::new(&[white, firework], 12);
    let mut frames = Vec::new();
    for color in up.spectrum().iter().chain(down.spectrum()) {
        for _ in 0..BLOSSOM_HOLD {
            frames.push(*color);
        }
    }
    if frames.is_empty() {
        frames.push(firework);
    }
    frames
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Waiting,
    Origin,
    Explode,
    Drop,
    Bloom,
    Done,
}

struct Shell {
    id: CharacterId,
    input_symbol: String,
    input_coord: Coord,
    final_color: Color,
    firework_color: Color,
    launch_pos: Coord,
    explode_at: Coord,
    explode_end: Coord,
    explode_ctrl: Coord,
    drop_mid: Coord,
    drop_ctrl: Coord,
    phase: Phase,
    progress: f64,
    pos: Coord,
    blossom: Vec<Color>,
    blossom_idx: usize,
}

impl Shell {
    fn busy(&self) -> bool {
        !matches!(self.phase, Phase::Waiting | Phase::Done)
    }

    fn visual(&self) -> (&str, Color) {
        match self.phase {
            Phase::Waiting | Phase::Origin => (FIREWORK_SYMBOL, self.firework_color),
            _ => {
                if self.blossom_idx < self.blossom.len() {
                    (self.input_symbol.as_str(), self.blossom[self.blossom_idx])
                } else {
                    (self.input_symbol.as_str(), self.final_color)
                }
            }
        }
    }

    fn step_blossom(&mut self) {
        if self.blossom_idx + 1 < self.blossom.len() {
            self.blossom_idx += 1;
        }
    }

    fn blossom_done(&self) -> bool {
        self.blossom.is_empty() || self.blossom_idx + 1 >= self.blossom.len()
    }

    fn step(&mut self) {
        match self.phase {
            Phase::Waiting | Phase::Done => {}
            Phase::Origin => {
                let len = find_length_of_line(self.launch_pos, self.explode_at);
                self.progress = advance(self.progress, ORIGIN_SPEED, len);
                self.pos = lerp_coord(self.launch_pos, self.explode_at, out_expo(self.progress));
                if self.progress >= 1.0 {
                    self.phase = Phase::Explode;
                    self.progress = 0.0;
                    self.pos = self.explode_at;
                    self.blossom_idx = 0;
                }
            }
            Phase::Explode => {
                let len = find_length_of_bezier_curve(self.explode_at, self.explode_ctrl, self.explode_end);
                self.progress = advance(self.progress, EXPLODE_SPEED, len);
                self.pos = find_coord_on_bezier_curve(
                    self.explode_at,
                    self.explode_ctrl,
                    self.explode_end,
                    out_circ(self.progress),
                );
                self.step_blossom();
                if self.progress >= 1.0 {
                    self.phase = Phase::Drop;
                    self.progress = 0.0;
                    self.pos = self.explode_end;
                }
            }
            Phase::Drop => {
                let l1 = find_length_of_bezier_curve(self.explode_end, self.drop_ctrl, self.drop_mid);
                let l2 = find_length_of_line(self.drop_mid, self.input_coord);
                let total = l1 + l2;
                self.progress = advance(self.progress, DROP_SPEED, total);
                let t = in_expo(self.progress);
                let dist = t * total.max(1e-9);
                self.pos = if dist <= l1 {
                    let local = if l1 < 1e-9 { 1.0 } else { (dist / l1).clamp(0.0, 1.0) };
                    find_coord_on_bezier_curve(self.explode_end, self.drop_ctrl, self.drop_mid, local)
                } else {
                    let local = if l2 < 1e-9 {
                        1.0
                    } else {
                        ((dist - l1) / l2).clamp(0.0, 1.0)
                    };
                    lerp_coord(self.drop_mid, self.input_coord, local)
                };
                self.step_blossom();
                if self.progress >= 1.0 {
                    self.pos = self.input_coord;
                    if self.blossom_done() {
                        self.phase = Phase::Done;
                    } else {
                        self.phase = Phase::Bloom;
                    }
                }
            }
            Phase::Bloom => {
                self.step_blossom();
                self.pos = self.input_coord;
                if self.blossom_done() {
                    self.phase = Phase::Done;
                }
            }
        }
    }
}

fn simulate(input: &str) -> Vec<String> {
    let mut term = Terminal::from_input(input, TerminalConfig::default());
    let infos: Vec<(CharacterId, String, Coord)> = term
        .get_characters()
        .iter()
        .map(|ch| (ch.id, ch.input_symbol.clone(), ch.input_coord))
        .collect();

    if infos.is_empty() {
        return vec![term.render_frame()];
    }

    let canvas_left = term.canvas.left;
    let canvas_right = term.canvas.right;
    let canvas_top = term.canvas.top;
    let canvas_bottom = term.canvas.bottom;
    let center = term.canvas.center();
    let launch_pos = Coord::new(center.column, canvas_bottom);
    let explode_dist = (canvas_right * EXPLODE_DISTANCE_PCT / 100).clamp(7, 30);
    let top_half_bottom = center.row.min(canvas_top);

    let min_row = infos.iter().map(|(_, _, c)| c.row).min().unwrap_or(canvas_bottom);
    let max_row = infos.iter().map(|(_, _, c)| c.row).max().unwrap_or(canvas_top);
    let row_span = (max_row - min_row).max(1) as f64;

    let firework_colors = [
        hex("88F7E2"),
        hex("44D492"),
        hex("F5EB67"),
        hex("FFA15C"),
        hex("FA233E"),
    ];
    let final_grad = Gradient::new(&[hex("8A008A"), hex("00D1FF"), hex("FFFFFF")], 12);

    term.hide_all();
    for ch in term.get_characters_mut() {
        ch.motion.current_coord = launch_pos;
        ch.animation
            .set_appearance(FIREWORK_SYMBOL, Some(ColorPair::fg(Color::rgb(0, 0, 0))));
    }

    let mut rng = Rng::new(0xF1A_E_010u64.wrapping_mul(infos.len() as u64 + 1));
    let volume = ((infos.len() as f64) * FIREWORK_VOLUME).floor() as usize;
    let volume = volume.max(1);

    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut offset = 0;
    while offset < infos.len() {
        let end = (offset + volume).min(infos.len());
        groups.push((offset..end).collect());
        offset = end;
    }

    let mut shells: Vec<Shell> = infos
        .iter()
        .map(|(id, symbol, coord)| {
            let t = (coord.row - min_row) as f64 / row_span;
            Shell {
                id: *id,
                input_symbol: symbol.clone(),
                input_coord: *coord,
                final_color: final_grad.mapped_color(t).unwrap_or(Color::rgb(255, 255, 255)),
                firework_color: firework_colors[0],
                launch_pos,
                explode_at: launch_pos,
                explode_end: *coord,
                explode_ctrl: center,
                drop_mid: Coord::new(coord.column, canvas_bottom),
                drop_ctrl: center,
                phase: Phase::Waiting,
                progress: 0.0,
                pos: launch_pos,
                blossom: Vec::new(),
                blossom_idx: 0,
            }
        })
        .collect();

    for group in &groups {
        let color = firework_colors[rng.usize(firework_colors.len())];
        let palette = blossom_palette(color);
        let explode_at = random_coord(
            &mut rng,
            canvas_left,
            canvas_right,
            top_half_bottom,
            canvas_top,
        );
        for &idx in group {
            let explode_end = random_near(&mut rng, explode_at, explode_dist);
            let shell = &mut shells[idx];
            shell.firework_color = color;
            shell.explode_at = explode_at;
            shell.explode_end = explode_end;
            shell.explode_ctrl = center;
            shell.drop_mid = Coord::new(explode_end.column, canvas_bottom);
            shell.drop_ctrl = Coord::new(center.column, explode_at.row);
            shell.blossom = palette.clone();
        }
    }

    let mut frames = Vec::new();
    let mut pending = 0usize;
    let mut launch_delay = 0i32;
    let max_frames = 2000 + groups.len() * 80 + 1000;

    loop {
        if pending < groups.len() && launch_delay == 0 {
            for &idx in &groups[pending] {
                let shell = &mut shells[idx];
                shell.phase = Phase::Origin;
                shell.progress = 0.0;
                shell.pos = launch_pos;
                term.set_character_visibility(shell.id, true);
            }
            pending += 1;
            launch_delay = LAUNCH_DELAY;
        }

        for shell in &mut shells {
            shell.step();
            if let Some(ch) = term.get_character_mut(shell.id) {
                ch.motion.current_coord = shell.pos;
                let (symbol, color) = shell.visual();
                ch.animation
                    .set_appearance(symbol, Some(ColorPair::fg(color)));
            }
        }

        launch_delay = (launch_delay - 1).max(0);
        frames.push(term.render_frame());

        let any_busy = shells.iter().any(Shell::busy);
        if pending >= groups.len() && !any_busy {
            break;
        }
        if frames.len() >= max_frames {
            break;
        }
    }

    if frames.is_empty() {
        frames.push(term.render_frame());
    }
    frames
}
