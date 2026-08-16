use std::collections::{HashMap, HashSet};

use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::character::{CharacterId, EffectCharacter};
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};
use crate::utils::round_half_even;

const RAIN_COLOR: Color = Color { r: 79, g: 143, b: 186 };
const RAIN_DIM: Color = Color { r: 40, g: 78, b: 108 };
const LIGHTNING: Color = Color { r: 255, g: 255, b: 204 };
const LIGHTNING_HOT: Color = Color { r: 255, g: 255, b: 255 };
const LIGHTNING_WARM: Color = Color { r: 255, g: 244, b: 176 };
const FINAL_STOP_A: Color = Color { r: 44, g: 82, b: 112 };
const FINAL_STOP_B: Color = Color { r: 197, g: 225, b: 245 };
const FINAL_STOP_C: Color = Color { r: 122, g: 163, b: 196 };
const FLASH_MAX: i32 = 10;
const HOLD_FRAMES: usize = 18;

pub struct Thunderstorm;

impl Thunderstorm {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Thunderstorm {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Thunderstorm {
    fn name(&self) -> &str {
        "thunderstorm"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        let mut rng = StormRng::from_input(input);

        let left = term.canvas.left;
        let right = term.canvas.right;
        let top = term.canvas.top;
        let bottom = term.canvas.bottom;
        let width = (right - left + 1).max(1);
        let height = (top - bottom + 1).max(1);

        let final_gradient = Gradient::new(&[FINAL_STOP_A, FINAL_STOP_B, FINAL_STOP_C], 12);
        let flash_gradient = Gradient::new(&[LIGHTNING_HOT, LIGHTNING, LIGHTNING_WARM, RAIN_COLOR], 5);
        let bolt_spectrum: Vec<Color> = flash_gradient.spectrum().to_vec();

        let (min_row, max_row) = text_row_extents(term.get_characters());
        let row_span = (max_row - min_row).max(1) as f64;

        let mut states: HashMap<CharacterId, CharState> = HashMap::new();
        for ch in term.get_characters() {
            let progress = if max_row == min_row {
                0.5
            } else {
                (ch.input_coord.row - min_row) as f64 / row_span
            };
            let final_color = final_gradient
                .mapped_color(progress)
                .unwrap_or(FINAL_STOP_B);
            states.insert(
                ch.id,
                CharState {
                    final_color,
                    flash: 0,
                    revealed: false,
                },
            );
        }

        term.hide_all();

        let mut rain: Vec<RainDrop> = Vec::new();
        let target_drops = ((width * height) / 18).clamp(8, 96) as usize;
        for _ in 0..target_drops / 2 {
            rain.push(RainDrop::spawn(left, right, bottom, top, &mut rng));
        }

        let mut bolts: Vec<Bolt> = Vec::new();
        let storm_len = (height * 8 + 100).clamp(90, 280);
        let mut cooldown = rng.gen_range(4, 12);
        let mut strikes_done = 0i32;
        let min_strikes = 3;
        let mut out = Vec::with_capacity((storm_len as usize) + HOLD_FRAMES + 8);

        for frame_i in 0..storm_len {
            spawn_rain(&mut rain, target_drops, left, right, bottom, top, &mut rng);
            step_rain(&mut rain, bottom, top, left, right, &mut rng);

            cooldown -= 1;
            let must_strike = strikes_done < min_strikes
                && frame_i > storm_len / 8
                && frame_i < storm_len - height - 8
                && cooldown <= 0;
            let chance_strike = cooldown <= 0
                && frame_i > 6
                && frame_i < storm_len - 12
                && rng.chance(0.045);
            if must_strike || chance_strike {
                bolts.push(Bolt::generate(&term.canvas, &mut rng));
                strikes_done += 1;
                cooldown = rng.gen_range(8, 18);
            }

            let mut hit_coords: HashSet<Coord> = HashSet::new();
            for bolt in &mut bolts {
                bolt.tick();
                for (coord, _) in bolt.cells.iter().take(bolt.revealed) {
                    hit_coords.insert(*coord);
                    hit_coords.insert(Coord::new(coord.column - 1, coord.row));
                    hit_coords.insert(Coord::new(coord.column + 1, coord.row));
                    hit_coords.insert(Coord::new(coord.column, coord.row - 1));
                    hit_coords.insert(Coord::new(coord.column, coord.row + 1));
                }
            }
            bolts.retain(|b| !b.finished());

            let ids: Vec<CharacterId> = term.get_characters().iter().map(|c| c.id).collect();
            let mut spark: Vec<CharacterId> = Vec::new();
            for id in &ids {
                let Some(ch) = term.get_character(*id) else {
                    continue;
                };
                if ch.input_symbol.trim().is_empty() {
                    continue;
                }
                let coord = ch.input_coord;
                if hit_coords.contains(&coord) {
                    spark.push(*id);
                }
                for drop in &rain {
                    if drop.column == coord.column && drop.row == coord.row {
                        if let Some(st) = states.get_mut(id) {
                            if !st.revealed {
                                st.revealed = true;
                            }
                        }
                    }
                }
            }
            for id in spark {
                if let Some(st) = states.get_mut(&id) {
                    st.revealed = true;
                    st.flash = FLASH_MAX;
                }
            }

            let mut spread: Vec<CharacterId> = Vec::new();
            for id in &ids {
                let flash = states.get(id).map(|s| s.flash).unwrap_or(0);
                if flash <= 5 {
                    continue;
                }
                if let Some(ch) = term.get_character(*id) {
                    if rng.chance(0.4) {
                        if let Some(n) = ch.neighbors.left {
                            spread.push(n);
                        }
                    }
                    if rng.chance(0.4) {
                        if let Some(n) = ch.neighbors.right {
                            spread.push(n);
                        }
                    }
                    if rng.chance(0.28) {
                        if let Some(n) = ch.neighbors.above {
                            spread.push(n);
                        }
                    }
                    if rng.chance(0.28) {
                        if let Some(n) = ch.neighbors.below {
                            spread.push(n);
                        }
                    }
                }
            }
            for id in spread {
                if let Some(st) = states.get_mut(&id) {
                    st.revealed = true;
                    if st.flash < FLASH_MAX - 2 {
                        st.flash = FLASH_MAX - 2;
                    }
                }
            }

            if frame_i % 3 == 0 {
                let hidden: Vec<CharacterId> = ids
                    .iter()
                    .copied()
                    .filter(|id| {
                        states.get(id).is_some_and(|s| !s.revealed)
                            && term
                                .get_character(*id)
                                .is_some_and(|c| !c.input_symbol.trim().is_empty())
                    })
                    .collect();
                if !hidden.is_empty() {
                    let pick = hidden[rng.gen_range(0, hidden.len() as i32) as usize];
                    if let Some(st) = states.get_mut(&pick) {
                        st.revealed = true;
                    }
                }
            }

            apply_character_styles(&mut term, &mut states, &flash_gradient);
            out.push(compose_frame(
                &mut term,
                &rain,
                &bolts,
                RAIN_COLOR,
                &bolt_spectrum,
            ));
        }

        for ch in term.get_characters() {
            if ch.input_symbol.trim().is_empty() {
                continue;
            }
            if let Some(st) = states.get_mut(&ch.id) {
                st.revealed = true;
                st.flash = 0;
            }
        }
        rain.clear();
        bolts.clear();
        apply_character_styles(&mut term, &mut states, &flash_gradient);
        let settled = compose_frame(&mut term, &rain, &bolts, RAIN_COLOR, &bolt_spectrum);
        for _ in 0..HOLD_FRAMES {
            out.push(settled.clone());
        }

        if out.is_empty() {
            apply_character_styles(&mut term, &mut states, &flash_gradient);
            out.push(compose_frame(
                &mut term,
                &rain,
                &bolts,
                RAIN_COLOR,
                &bolt_spectrum,
            ));
        }
        out
    }
}

struct CharState {
    final_color: Color,
    flash: i32,
    revealed: bool,
}

struct RainDrop {
    column: i32,
    row: i32,
    wait: i32,
    delay: i32,
    symbol: &'static str,
}

impl RainDrop {
    fn spawn(left: i32, right: i32, bottom: i32, top: i32, rng: &mut StormRng) -> Self {
        let symbols = ["|", "│", ".", "'", ":"];
        Self {
            column: rng.gen_range(left, right + 1),
            row: rng.gen_range(bottom, top + 1),
            wait: rng.gen_range(0, 3),
            delay: rng.gen_range(0, 3),
            symbol: symbols[rng.gen_range(0, symbols.len() as i32) as usize],
        }
    }
}

struct Bolt {
    cells: Vec<(Coord, &'static str)>,
    revealed: usize,
    age: i32,
    max_age: i32,
    grow: usize,
}

impl Bolt {
    fn generate(canvas: &crate::engine::canvas::Canvas, rng: &mut StormRng) -> Self {
        let mut cells = Vec::new();
        let mut forks: Vec<(i32, i32, i32)> = Vec::new();
        let mut column = rng.gen_range(canvas.left, canvas.right + 1);
        let mut row = canvas.top;
        while row >= canvas.bottom {
            let choice = rng.gen_range(0, 3);
            let (symbol, dcol) = match choice {
                0 => ("/", -1),
                1 => ("\\", 1),
                _ => ("|", 0),
            };
            column = (column + dcol).clamp(canvas.left, canvas.right);
            cells.push((Coord::new(column, row), symbol));
            if rng.gen_range(0, 9) == 0 && row < canvas.top - 1 && row > canvas.bottom + 1 {
                let dir = if dcol <= 0 { 1 } else { -1 };
                forks.push((column, row, dir));
            }
            row -= 1;
        }
        for (mut fcol, frow, dir) in forks {
            let mut frow = frow - 1;
            let depth = rng.gen_range(3, 9);
            for _ in 0..depth {
                if frow < canvas.bottom {
                    break;
                }
                fcol = (fcol + dir).clamp(canvas.left, canvas.right);
                let symbol = if dir > 0 { "\\" } else { "/" };
                cells.push((Coord::new(fcol, frow), symbol));
                frow -= 1;
                if rng.gen_range(0, 3) == 0 && frow >= canvas.bottom {
                    cells.push((Coord::new(fcol, frow), "|"));
                    frow -= 1;
                }
            }
        }
        let len = cells.len() as i32;
        Self {
            cells,
            revealed: 0,
            age: 0,
            max_age: (8 + len / 3).max(10),
            grow: (2 + rng.gen_range(0, 4)) as usize,
        }
    }

    fn tick(&mut self) {
        self.revealed = (self.revealed + self.grow).min(self.cells.len());
        self.age += 1;
    }

    fn finished(&self) -> bool {
        self.age >= self.max_age
    }
}

struct StormRng(u32);

impl StormRng {
    fn from_input(input: &str) -> Self {
        let mut seed = 0x5468_756eu32;
        for b in input.as_bytes() {
            seed = seed.wrapping_mul(16_777_619) ^ u32::from(*b);
        }
        if seed == 0 {
            seed = 0xC0FF_EE01;
        }
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    fn gen_range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u32() as i32).rem_euclid(hi - lo)
    }

    fn chance(&mut self, p: f64) -> bool {
        (f64::from(self.next_u32()) / f64::from(u32::MAX)) < p
    }
}

fn text_row_extents(chars: &[EffectCharacter]) -> (i32, i32) {
    let mut min_row = i32::MAX;
    let mut max_row = i32::MIN;
    for ch in chars {
        min_row = min_row.min(ch.input_coord.row);
        max_row = max_row.max(ch.input_coord.row);
    }
    if min_row == i32::MAX {
        (1, 1)
    } else {
        (min_row, max_row)
    }
}

fn spawn_rain(
    rain: &mut Vec<RainDrop>,
    target: usize,
    left: i32,
    right: i32,
    bottom: i32,
    top: i32,
    rng: &mut StormRng,
) {
    let extra = rng.gen_range(1, 4) as usize;
    while rain.len() < target {
        let mut drop = RainDrop::spawn(left, right, bottom, top, rng);
        drop.row = top;
        rain.push(drop);
        if rain.len() >= target {
            break;
        }
    }
    for _ in 0..extra {
        if rain.len() >= target + extra {
            break;
        }
        let mut drop = RainDrop::spawn(left, right, bottom, top, rng);
        drop.row = top;
        rain.push(drop);
    }
}

fn step_rain(
    rain: &mut Vec<RainDrop>,
    bottom: i32,
    top: i32,
    left: i32,
    right: i32,
    rng: &mut StormRng,
) {
    for drop in rain.iter_mut() {
        if drop.wait > 0 {
            drop.wait -= 1;
            continue;
        }
        drop.row -= 1;
        drop.wait = drop.delay;
        if drop.row < bottom {
            drop.column = rng.gen_range(left, right + 1);
            drop.row = top;
            drop.delay = rng.gen_range(0, 3);
            drop.wait = drop.delay;
            let symbols = ["|", "│", ".", "'", ":"];
            drop.symbol = symbols[rng.gen_range(0, symbols.len() as i32) as usize];
        }
    }
}

fn mix(a: Color, b: Color, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::rgb(
        round_half_even(f64::from(a.r) + (f64::from(b.r) - f64::from(a.r)) * t).clamp(0, 255) as u8,
        round_half_even(f64::from(a.g) + (f64::from(b.g) - f64::from(a.g)) * t).clamp(0, 255) as u8,
        round_half_even(f64::from(a.b) + (f64::from(b.b) - f64::from(a.b)) * t).clamp(0, 255) as u8,
    )
}

fn paint(ch: &mut EffectCharacter, color: Color, flash: bool) {
    let painted = format!("{}{}\x1b[0m", color.fg_sgr(), ch.input_symbol);
    ch.animation.set_appearance(&painted, Some(ColorPair::fg(color)));
    ch.animation.current_character_visual.symbol = painted;
    ch.animation.current_character_visual.bold = flash;
    ch.animation.current_character_visual.dim = !flash;
    ch.animation.current_character_visual.refresh();
    ch.is_visible = true;
}

fn apply_character_styles(
    term: &mut Terminal,
    states: &mut HashMap<CharacterId, CharState>,
    flash_gradient: &Gradient,
) {
    let ids: Vec<CharacterId> = term.get_characters().iter().map(|c| c.id).collect();
    for id in ids {
        let Some(ch) = term.get_character_mut(id) else {
            continue;
        };
        if ch.input_symbol.trim().is_empty() {
            ch.is_visible = false;
            if let Some(st) = states.get_mut(&id) {
                if st.flash > 0 {
                    st.flash -= 1;
                }
            }
            continue;
        }
        let Some(st) = states.get_mut(&id) else {
            continue;
        };
        if !st.revealed {
            ch.is_visible = false;
            continue;
        }
        let color = if st.flash > 0 {
            let progress = 1.0 - f64::from(st.flash) / f64::from(FLASH_MAX);
            let bolt = flash_gradient
                .mapped_color(progress)
                .unwrap_or(LIGHTNING);
            let toward_final = (1.0 - f64::from(st.flash) / f64::from(FLASH_MAX)).clamp(0.0, 1.0);
            st.flash -= 1;
            mix(bolt, st.final_color, toward_final * 0.35)
        } else {
            mix(RAIN_DIM, st.final_color, 0.85)
        };
        let flashing = st.flash > 2;
        paint(ch, color, flashing);
    }
}

fn compose_frame(
    term: &mut Terminal,
    rain: &[RainDrop],
    bolts: &[Bolt],
    rain_color: Color,
    bolt_colors: &[Color],
) -> String {
    let _ = term.render_frame();
    let occupied: HashSet<Coord> = term
        .get_characters()
        .iter()
        .filter(|ch| ch.is_visible)
        .map(|ch| ch.current_coord())
        .collect();

    for drop in rain {
        let coord = Coord::new(drop.column, drop.row);
        if !term.canvas.contains(coord) || occupied.contains(&coord) {
            continue;
        }
        let glyph = format!("{}{}\x1b[0m", rain_color.fg_sgr(), drop.symbol);
        term.canvas
            .put(coord, CharacterVisual::new(glyph.as_str(), None));
    }

    for bolt in bolts {
        let fade = if bolt.max_age <= 1 {
            0.0
        } else {
            f64::from(bolt.age) / f64::from(bolt.max_age)
        };
        let color = if bolt_colors.is_empty() {
            LIGHTNING
        } else {
            let last = bolt_colors.len() - 1;
            let idx = round_half_even(fade.clamp(0.0, 1.0) * last as f64).clamp(0, last as i64) as usize;
            bolt_colors[idx]
        };
        let bright = color.adjust_brightness(1.15);
        for (coord, symbol) in bolt.cells.iter().take(bolt.revealed) {
            if !term.canvas.contains(*coord) {
                continue;
            }
            let glyph = format!("{}{}\x1b[0m", bright.fg_sgr(), symbol);
            term.canvas
                .put(*coord, CharacterVisual::new(glyph.as_str(), None));
        }
    }

    term.canvas.render()
}
