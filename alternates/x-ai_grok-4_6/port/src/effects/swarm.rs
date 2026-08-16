//! Characters are grouped into swarms and fly around the canvas before
//! settling into their input positions. Port of TTE `effect_swarm`.

use super::Effect;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{
    find_coord_on_bezier_curve, find_coords_in_circle, find_length_of_bezier_curve, Coord,
};
use crate::utils::graphics::{Color, ColorPair, Gradient};
use crate::utils::round_half_even;

const SWARM_SIZE_RATIO: f64 = 0.1;
const SWARM_COORDINATION: f64 = 0.80;
const AREA_COUNT_MIN: i32 = 2;
const AREA_COUNT_MAX: i32 = 4;
const FLIGHT_SPEED: f64 = 0.25;
const FADE_HOLD: u32 = 10;
const FADE_STEPS: usize = 10;
const SWARM_GRAD_STEPS: usize = 7;
const FINAL_GRAD_STEPS: usize = 12;
const MAX_FRAMES: usize = 20_000;

const FLASH: Color = Color { r: 255, g: 255, b: 255 };
const BASE: Color = Color { r: 0x44, g: 0xaa, b: 0xee };
const FINAL_STOP_A: Color = Color { r: 0x44, g: 0xaa, b: 0xee };
const FINAL_STOP_B: Color = Color { r: 0x00, g: 0x00, b: 0xff };

pub struct Swarm;

impl Swarm {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Swarm {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Swarm {
    fn name(&self) -> &str {
        "swarm"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        term.hide_all();

        let n = term.character_count();
        if n == 0 {
            return vec![term.render_frame()];
        }

        let canvas_left = term.canvas.left;
        let canvas_right = term.canvas.right;
        let canvas_top = term.canvas.top;
        let canvas_bottom = term.canvas.bottom;
        let canvas_center = term.canvas.center();

        let snaps: Vec<Snap> = term
            .get_characters()
            .iter()
            .map(|ch| Snap {
                symbol: ch.input_symbol.clone(),
                home: ch.input_coord,
            })
            .collect();

        let text_left = snaps.iter().map(|s| s.home.column).min().unwrap_or(canvas_left);
        let text_right = snaps.iter().map(|s| s.home.column).max().unwrap_or(canvas_right);

        let final_gradient = Gradient::new(&[FINAL_STOP_A, FINAL_STOP_B], FINAL_GRAD_STEPS);

        let mut rng = Rng::from_input(input);
        let swarm_size = round_half_even(n as f64 * SWARM_SIZE_RATIO).max(1) as usize;

        let mut unswarmed: Vec<usize> = (0..n).rev().collect();
        let mut swarms: Vec<Vec<usize>> = Vec::new();
        while !unswarmed.is_empty() {
            let mut swarm = Vec::new();
            for _ in 0..swarm_size {
                if unswarmed.is_empty() {
                    break;
                }
                let pick = rng.index(unswarmed.len());
                swarm.push(unswarmed.remove(pick));
            }
            swarms.push(swarm);
        }

        let mut flyers: Vec<Flyer> = snaps
            .iter()
            .map(|s| {
                let final_color = map_horizontal(&final_gradient, s.home.column, text_left, text_right);
                Flyer::placeholder(s.symbol.clone(), s.home, final_color)
            })
            .collect();

        for swarm in &swarms {
            let spawn = random_outside(
                &mut rng,
                canvas_left,
                canvas_right,
                canvas_top,
                canvas_bottom,
            );
            let area_count = rng.inclusive(AREA_COUNT_MIN, AREA_COUNT_MAX) as usize;
            let mut areas = Vec::with_capacity(area_count);
            for _ in 0..area_count {
                areas.push(random_area(
                    &mut rng,
                    canvas_center,
                    canvas_right,
                ));
            }
            let swarm_grad = Gradient::new(&[FLASH, BASE], SWARM_GRAD_STEPS);
            for &idx in swarm {
                let home = flyers[idx].home;
                let wander = rng.unit() > SWARM_COORDINATION;
                let mut points = Vec::new();
                let mut controls = Vec::new();
                if wander {
                    for _ in 0..areas.len() {
                        points.push(random_inside(
                            &mut rng,
                            canvas_left,
                            canvas_right,
                            canvas_top,
                            canvas_bottom,
                        ));
                        controls.push(random_inside(
                            &mut rng,
                            canvas_left,
                            canvas_right,
                            canvas_top,
                            canvas_bottom,
                        ));
                    }
                } else {
                    for &area in &areas {
                        points.push(area);
                        controls.push(random_inside(
                            &mut rng,
                            canvas_left,
                            canvas_right,
                            canvas_top,
                            canvas_bottom,
                        ));
                    }
                }
                points.push(home);
                controls.push(random_inside(
                    &mut rng,
                    canvas_left,
                    canvas_right,
                    canvas_top,
                    canvas_bottom,
                ));
                flyers[idx].configure(spawn, points, controls, swarm_grad.clone());
            }
        }

        let mut frames = Vec::new();
        let mut next_swarm = 0usize;

        loop {
            if frames.len() >= MAX_FRAMES {
                break;
            }
            let any_busy = flyers
                .iter()
                .any(|f| matches!(f.phase, Phase::Flying | Phase::Fading));
            if !any_busy {
                if next_swarm >= swarms.len() {
                    break;
                }
                for &idx in &swarms[next_swarm] {
                    flyers[idx].launch();
                }
                next_swarm += 1;
            }

            let mut poses: Vec<(Coord, Color, bool)> = Vec::with_capacity(flyers.len());
            for flyer in flyers.iter_mut() {
                if flyer.phase == Phase::Waiting {
                    poses.push((flyer.spawn, FLASH, false));
                } else {
                    let (pos, color) = flyer.advance();
                    poses.push((pos, color, true));
                }
            }

            {
                let chars = term.get_characters_mut();
                for (ch, (pos, color, visible)) in chars.iter_mut().zip(poses.iter()) {
                    ch.is_visible = *visible;
                    if *visible {
                        ch.motion.current_coord = *pos;
                        ch.animation
                            .set_appearance(&ch.input_symbol, Some(ColorPair::fg(*color)));
                    }
                }
            }
            frames.push(term.render_frame());
        }

        if frames.is_empty() {
            term.show_all();
            frames.push(term.render_frame());
        }
        frames
    }
}

struct Snap {
    symbol: String,
    home: Coord,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Waiting,
    Flying,
    Fading,
    Done,
}

struct Flyer {
    home: Coord,
    spawn: Coord,
    points: Vec<Coord>,
    controls: Vec<Coord>,
    seg: usize,
    t: f64,
    final_color: Color,
    swarm_grad: Gradient,
    fade_grad: Gradient,
    fade_tick: u32,
    phase: Phase,
}

impl Flyer {
    fn placeholder(symbol: String, home: Coord, final_color: Color) -> Self {
        let _ = symbol;
        let swarm_grad = Gradient::new(&[FLASH, BASE], SWARM_GRAD_STEPS);
        let fade_grad = Gradient::new(&[BASE, final_color], FADE_STEPS);
        Self {
            home,
            spawn: home,
            points: vec![home],
            controls: vec![home],
            seg: 0,
            t: 0.0,
            final_color,
            swarm_grad,
            fade_grad,
            fade_tick: 0,
            phase: Phase::Waiting,
        }
    }

    fn configure(
        &mut self,
        spawn: Coord,
        points: Vec<Coord>,
        controls: Vec<Coord>,
        swarm_grad: Gradient,
    ) {
        let last_swarm = swarm_grad
            .get(swarm_grad.len().saturating_sub(1))
            .unwrap_or(BASE);
        self.fade_grad = Gradient::new(&[last_swarm, self.final_color], FADE_STEPS);
        self.spawn = spawn;
        self.points = points;
        self.controls = controls;
        self.swarm_grad = swarm_grad;
    }

    fn launch(&mut self) {
        self.phase = Phase::Flying;
        self.seg = 0;
        self.t = 0.0;
        self.fade_tick = 0;
    }

    fn advance(&mut self) -> (Coord, Color) {
        match self.phase {
            Phase::Waiting => (self.spawn, FLASH),
            Phase::Flying => self.advance_flight(),
            Phase::Fading => self.advance_fade(),
            Phase::Done => (self.home, self.final_color),
        }
    }

    fn advance_flight(&mut self) -> (Coord, Color) {
        if self.seg >= self.points.len() {
            self.phase = Phase::Fading;
            self.fade_tick = 0;
            return self.advance_fade();
        }
        let start = if self.seg == 0 {
            self.spawn
        } else {
            self.points[self.seg - 1]
        };
        let end = self.points[self.seg];
        let control = self
            .controls
            .get(self.seg)
            .copied()
            .unwrap_or(end);
        let len = find_length_of_bezier_curve(start, control, end);
        if len < 0.0001 {
            self.seg += 1;
            self.t = 0.0;
            return self.flight_sample(end);
        }
        self.t += FLIGHT_SPEED / len;
        if self.t >= 1.0 {
            self.seg += 1;
            self.t = 0.0;
            if self.seg >= self.points.len() {
                self.phase = Phase::Fading;
                self.fade_tick = 0;
                return self.advance_fade();
            }
            return self.flight_sample(end);
        }
        let eased = in_out_sine(self.t);
        let pos = find_coord_on_bezier_curve(start, control, end, eased);
        self.flight_sample(pos)
    }

    fn flight_sample(&self, pos: Coord) -> (Coord, Color) {
        let segs = self.points.len().max(1) as f64;
        let progress = ((self.seg as f64) + self.t.clamp(0.0, 1.0)) / segs;
        let color = self
            .swarm_grad
            .mapped_color(progress.clamp(0.0, 1.0))
            .unwrap_or(FLASH);
        (pos, color)
    }

    fn advance_fade(&mut self) -> (Coord, Color) {
        let idx = (self.fade_tick / FADE_HOLD) as usize;
        self.fade_tick = self.fade_tick.saturating_add(1);
        if idx >= self.fade_grad.len() {
            self.phase = Phase::Done;
            return (self.home, self.final_color);
        }
        let color = self.fade_grad.get(idx).unwrap_or(self.final_color);
        (self.home, color)
    }
}

fn in_out_sine(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    -((t * std::f64::consts::PI).cos() - 1.0) / 2.0
}

fn map_horizontal(grad: &Gradient, column: i32, left: i32, right: i32) -> Color {
    if right <= left {
        return grad.mapped_color(0.0).unwrap_or(FINAL_STOP_A);
    }
    let t = f64::from(column - left) / f64::from(right - left);
    grad.mapped_color(t).unwrap_or(FINAL_STOP_A)
}

fn random_inside(rng: &mut Rng, left: i32, right: i32, top: i32, bottom: i32) -> Coord {
    Coord::new(rng.inclusive(left, right), rng.inclusive(bottom, top))
}

fn random_outside(rng: &mut Rng, left: i32, right: i32, top: i32, bottom: i32) -> Coord {
    match rng.inclusive(0, 3) {
        0 => Coord::new(rng.inclusive(left, right), top + 1),
        1 => Coord::new(rng.inclusive(left, right), bottom - 1),
        2 => Coord::new(left - 1, rng.inclusive(bottom, top)),
        _ => Coord::new(right + 1, rng.inclusive(bottom, top)),
    }
}

fn random_area(rng: &mut Rng, center: Coord, canvas_right: i32) -> Coord {
    let inner = (canvas_right / 2).min(4).max(0);
    let outer = canvas_right.min(10).max(inner);
    let radius = rng.inclusive(inner, outer) as f64;
    let coords = find_coords_in_circle(center, radius);
    if coords.is_empty() {
        center
    } else {
        coords[rng.index(coords.len())]
    }
}

struct Rng {
    state: u64,
}

impl Rng {
    fn from_input(input: &str) -> Self {
        let mut seed: u64 = 0x7377_6172_6d5f_7474;
        for b in input.bytes() {
            seed = seed.wrapping_mul(131).wrapping_add(u64::from(b));
        }
        Self { state: seed | 1 }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        (x >> 32) as u32
    }

    fn unit(&mut self) -> f64 {
        f64::from(self.next_u32()) / f64::from(u32::MAX)
    }

    fn inclusive(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let span = (i64::from(hi) - i64::from(lo) + 1) as u32;
        lo.saturating_add((self.next_u32() % span) as i32)
    }

    fn index(&mut self, len: usize) -> usize {
        if len <= 1 {
            0
        } else {
            (self.next_u32() as usize) % len
        }
    }
}
