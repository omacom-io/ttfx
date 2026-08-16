use super::Effect;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{distance, lerp_coord, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

const EXPLOSION_SPEED: f64 = 0.75;
const REASSEMBLY_SPEED: f64 = 0.75;
const RUMBLE_FRAMES: i32 = 50;
const RUMBLE_COLOR_HOLD: u32 = 10;
const FINAL_COLOR_HOLD: u32 = 5;
const RUMBLE_GRADIENT_STEPS: usize = 25;
const FINAL_GRADIENT_STEPS: usize = 12;
const MAX_FRAMES: usize = 4000;

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
        let mut terminal = Terminal::from_input(input, TerminalConfig::default());
        terminal.show_all();

        if terminal.character_count() == 0 {
            return vec![terminal.render_frame()];
        }

        let left = terminal.canvas.left;
        let right = terminal.canvas.right;
        let top = terminal.canvas.top;
        let bottom = terminal.canvas.bottom;

        let unstable = hex_color("ff9200", 255, 146, 0);
        let stops = [
            hex_color("8A008A", 138, 0, 138),
            hex_color("00D1FF", 0, 209, 255),
            hex_color("FFFFFF", 255, 255, 255),
        ];
        let final_gradient = Gradient::new(&stops, FINAL_GRADIENT_STEPS);

        let text_left = terminal
            .get_characters()
            .iter()
            .map(|ch| ch.input_coord.column)
            .min()
            .unwrap_or(left);
        let text_right = terminal
            .get_characters()
            .iter()
            .map(|ch| ch.input_coord.column)
            .max()
            .unwrap_or(right);
        let text_top = terminal
            .get_characters()
            .iter()
            .map(|ch| ch.input_coord.row)
            .max()
            .unwrap_or(top);
        let text_bottom = terminal
            .get_characters()
            .iter()
            .map(|ch| ch.input_coord.row)
            .min()
            .unwrap_or(bottom);
        let _ = (text_left, text_right);
        let row_span = f64::from((text_top - text_bottom).max(1));

        let mut rng = SimpleRng::new(0x00C0_FFEE_u64);
        let mut pool: Vec<Coord> = terminal
            .get_characters()
            .iter()
            .map(|ch| ch.input_coord)
            .collect();
        for i in (1..pool.len()).rev() {
            let j = rng.index(i + 1);
            pool.swap(i, j);
        }

        let mut actors: Vec<Actor> = terminal
            .get_characters()
            .iter()
            .enumerate()
            .map(|(idx, ch)| {
                let jumbled = pool[idx];
                let blast = match rng.index(4) {
                    0 => Coord::new(left, rng.inclusive(bottom, top)),
                    1 => Coord::new(right, rng.inclusive(bottom, top)),
                    2 => Coord::new(rng.inclusive(left, right), bottom),
                    _ => Coord::new(rng.inclusive(left, right), top),
                };
                let progress = f64::from(text_top - ch.input_coord.row) / row_span;
                let final_color = final_gradient.mapped_color(progress).unwrap_or(unstable);
                Actor {
                    id: ch.id,
                    symbol: ch.input_symbol.clone(),
                    home: ch.input_coord,
                    jumbled,
                    blast,
                    bg: ch.input_bg,
                    rumble_spectrum: Gradient::new(&[final_color, unstable], RUMBLE_GRADIENT_STEPS)
                        .spectrum()
                        .to_vec(),
                    final_spectrum: Gradient::new(&[unstable, final_color], FINAL_GRADIENT_STEPS)
                        .spectrum()
                        .to_vec(),
                    traveled: 0.0,
                    return_tick: 0,
                    stage: Stage::Hold,
                }
            })
            .collect();

        for actor in &actors {
            paint(
                &mut terminal,
                actor,
                actor.jumbled,
                spectrum_color(&actor.rumble_spectrum, 0, RUMBLE_COLOR_HOLD),
            );
        }

        let mut frames = Vec::new();
        let mut rumble_left = RUMBLE_FRAMES;
        let mut anim_tick = 0u32;

        for _ in 0..MAX_FRAMES {
            if rumble_left > 0 {
                for actor in &actors {
                    if let Some(ch) = terminal.get_character_mut(actor.id) {
                        ch.motion.current_coord.column += rng.inclusive(-1, 1);
                        ch.motion.current_coord.row += rng.inclusive(-1, 1);
                        ch.animation.set_appearance(
                            &actor.symbol,
                            Some(ColorPair::new(
                                Some(spectrum_color(
                                    &actor.rumble_spectrum,
                                    anim_tick,
                                    RUMBLE_COLOR_HOLD,
                                )),
                                actor.bg,
                            )),
                        );
                    }
                }
                rumble_left -= 1;
                anim_tick = anim_tick.saturating_add(1);
                frames.push(terminal.render_frame());
                continue;
            }

            let mut all_rest = true;
            for actor in &mut actors {
                match actor.stage {
                    Stage::Hold => {
                        actor.stage = Stage::Out;
                        actor.traveled = 0.0;
                        if let Some(ch) = terminal.get_character_mut(actor.id) {
                            ch.motion.current_coord = actor.jumbled;
                        }
                        step_out(actor, &mut terminal, anim_tick);
                        all_rest = false;
                    }
                    Stage::Out => {
                        step_out(actor, &mut terminal, anim_tick);
                        all_rest = false;
                    }
                    Stage::Back => {
                        step_back(actor, &mut terminal);
                        if actor.stage != Stage::Rest {
                            all_rest = false;
                        }
                    }
                    Stage::Rest => {}
                }
            }
            anim_tick = anim_tick.saturating_add(1);
            frames.push(terminal.render_frame());
            if all_rest {
                break;
            }
        }

        if frames.is_empty() {
            frames.push(terminal.render_frame());
        }
        frames
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Hold,
    Out,
    Back,
    Rest,
}

struct Actor {
    id: CharacterId,
    symbol: String,
    home: Coord,
    jumbled: Coord,
    blast: Coord,
    bg: Option<Color>,
    rumble_spectrum: Vec<Color>,
    final_spectrum: Vec<Color>,
    traveled: f64,
    return_tick: u32,
    stage: Stage,
}

fn step_out(actor: &mut Actor, terminal: &mut Terminal, anim_tick: u32) {
    let len = distance(actor.jumbled, actor.blast);
    actor.traveled += EXPLOSION_SPEED;
    let done = len <= 0.0 || actor.traveled >= len;
    let t = if len <= 0.0 {
        1.0
    } else {
        (actor.traveled / len).clamp(0.0, 1.0)
    };
    let coord = lerp_coord(actor.jumbled, actor.blast, out_expo(t));
    paint(
        terminal,
        actor,
        coord,
        spectrum_color(&actor.rumble_spectrum, anim_tick, RUMBLE_COLOR_HOLD),
    );
    if done {
        actor.stage = Stage::Back;
        actor.traveled = 0.0;
        actor.return_tick = 0;
    }
}

fn step_back(actor: &mut Actor, terminal: &mut Terminal) {
    let len = distance(actor.blast, actor.home);
    actor.traveled += REASSEMBLY_SPEED;
    let move_done = len <= 0.0 || actor.traveled >= len;
    let t = if len <= 0.0 {
        1.0
    } else {
        (actor.traveled / len).clamp(0.0, 1.0)
    };
    let coord = lerp_coord(actor.blast, actor.home, out_expo(t));
    paint(
        terminal,
        actor,
        coord,
        spectrum_color(&actor.final_spectrum, actor.return_tick, FINAL_COLOR_HOLD),
    );
    let color_frames = (actor.final_spectrum.len() as u32).saturating_mul(FINAL_COLOR_HOLD).max(1);
    actor.return_tick = actor.return_tick.saturating_add(1);
    if move_done && actor.return_tick >= color_frames {
        actor.stage = Stage::Rest;
        paint(
            terminal,
            actor,
            actor.home,
            spectrum_color(&actor.final_spectrum, u32::MAX, FINAL_COLOR_HOLD),
        );
    }
}

fn paint(terminal: &mut Terminal, actor: &Actor, coord: Coord, fg: Color) {
    if let Some(ch) = terminal.get_character_mut(actor.id) {
        ch.motion.current_coord = coord;
        ch.animation
            .set_appearance(&actor.symbol, Some(ColorPair::new(Some(fg), actor.bg)));
    }
}

fn spectrum_color(spectrum: &[Color], tick: u32, hold: u32) -> Color {
    if spectrum.is_empty() {
        return Color::rgb(255, 255, 255);
    }
    let idx = (tick / hold.max(1)) as usize;
    spectrum[idx.min(spectrum.len() - 1)]
}

fn out_expo(t: f64) -> f64 {
    if t >= 1.0 {
        1.0
    } else if t <= 0.0 {
        0.0
    } else {
        1.0 - 2f64.powf(-10.0 * t)
    }
}

fn hex_color(hex: &str, r: u8, g: u8, b: u8) -> Color {
    Color::from_hex(hex).unwrap_or(Color::rgb(r, g, b))
}

struct SimpleRng(u64);

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }

    fn inclusive(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let span = (i64::from(hi) - i64::from(lo) + 1) as u32;
        lo.saturating_add((self.next() % span) as i32)
    }

    fn index(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() as usize) % n
        }
    }
}
