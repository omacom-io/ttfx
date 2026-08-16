use std::collections::VecDeque;

use super::Effect;
use crate::engine::animation::CharacterVisual;
use crate::engine::character::CharacterId;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::geometry::{distance, find_coords_on_line, lerp_coord, Coord};
use crate::utils::graphics::{Color, ColorPair, Gradient};

const LAUNCHER_SPEED: f64 = 0.5;
const CHARACTER_SPEED: f64 = 1.0;
const VOLLEY_SIZE: f64 = 0.03;
const LAUNCH_DELAY: i32 = 3;
const GRADIENT_STEPS: usize = 12;
const TOP_LAUNCHER_SYMBOL: &str = "█";
const BOTTOM_LAUNCHER_SYMBOL: &str = "█";
const MAX_FRAMES: usize = 20_000;

pub struct Orbittingvolley;

impl Orbittingvolley {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Orbittingvolley {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Orbittingvolley {
    fn name(&self) -> &str {
        "orbittingvolley"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut term = Terminal::from_input(input, TerminalConfig::default());
        if term.character_count() == 0 {
            return vec![term.render_frame()];
        }

        let stops = [
            Color::rgb(0x8a, 0x00, 0x8a),
            Color::rgb(0x00, 0xd1, 0xff),
            Color::rgb(0xff, 0xff, 0xff),
        ];
        let gradient = Gradient::new(&stops, GRADIENT_STEPS);

        let (text_bottom, text_top) = {
            let chars = term.get_characters();
            let bottom = chars.iter().map(|c| c.input_coord.row).min().unwrap_or(1);
            let top = chars.iter().map(|c| c.input_coord.row).max().unwrap_or(1);
            (bottom, top)
        };

        for ch in term.get_characters_mut() {
            let progress = if text_top == text_bottom {
                1.0
            } else {
                f64::from(ch.input_coord.row - text_bottom)
                    / f64::from(text_top - text_bottom)
            };
            let color = gradient
                .mapped_color(progress)
                .unwrap_or(Color::rgb(0xff, 0xff, 0xff));
            let symbol = ch.input_symbol.clone();
            ch.animation
                .set_appearance(&symbol, Some(ColorPair::fg(color)));
        }

        let mut order: Vec<(i32, i32, CharacterId)> = term
            .get_characters()
            .iter()
            .map(|c| (c.input_coord.row, c.input_coord.column, c.id))
            .collect();
        order.sort_unstable();

        let mut top_mag: VecDeque<CharacterId> = VecDeque::new();
        let mut bot_mag: VecDeque<CharacterId> = VecDeque::new();
        for (i, &(_, _, id)) in order.iter().enumerate() {
            if i % 2 == 0 {
                top_mag.push_back(id);
            } else {
                bot_mag.push_back(id);
            }
        }

        let top_start = Coord::new(term.canvas.left, term.canvas.top);
        let bot_start = Coord::new(term.canvas.right, term.canvas.bottom);
        let mut top = Launcher::new(
            top_start,
            perimeter_path(top_start, &term.canvas),
            top_mag,
            launcher_visual(TOP_LAUNCHER_SYMBOL),
        );
        let mut bot = Launcher::new(
            bot_start,
            perimeter_path(bot_start, &term.canvas),
            bot_mag,
            launcher_visual(BOTTOM_LAUNCHER_SYMBOL),
        );

        let n = term.character_count();
        let volley = ((VOLLEY_SIZE * n as f64) as usize).max(1);
        let mut flying: Vec<Flyer> = Vec::new();
        let mut delay: i32 = 0;
        let mut frames: Vec<String> = Vec::new();

        for _ in 0..MAX_FRAMES {
            let ammo = !top.magazine.is_empty() || !bot.magazine.is_empty();
            let in_flight = flying.iter().any(|f| !f.done);
            let launchers_moving = !top.finished() || !bot.finished();
            if !ammo && !in_flight && !launchers_moving {
                break;
            }

            if delay == 0 {
                let top_origin = top.pos();
                let bot_origin = bot.pos();
                launch_from(&mut top.magazine, top_origin, volley, &mut term, &mut flying);
                launch_from(&mut bot.magazine, bot_origin, volley, &mut term, &mut flying);
                delay = LAUNCH_DELAY;
            } else {
                delay -= 1;
            }

            top.step();
            bot.step();

            for flyer in &mut flying {
                if flyer.done {
                    continue;
                }
                flyer.traveled += CHARACTER_SPEED;
                let t = if flyer.total <= 0.0 {
                    1.0
                } else {
                    (flyer.traveled / flyer.total).clamp(0.0, 1.0)
                };
                let pos = lerp_coord(flyer.start, flyer.end, out_sine(t));
                if let Some(ch) = term.get_character_mut(flyer.id) {
                    ch.motion.current_coord = pos;
                }
                if t >= 1.0 {
                    flyer.done = true;
                }
            }

            frames.push(paint_frame(&mut term, &top, &bot));
        }

        if frames.is_empty() {
            term.show_all();
            for ch in term.get_characters_mut() {
                ch.motion.current_coord = ch.input_coord;
            }
            frames.push(term.render_frame());
        }

        frames
    }
}

struct Launcher {
    path: Vec<Coord>,
    dist: f64,
    magazine: VecDeque<CharacterId>,
    visual: CharacterVisual,
}

impl Launcher {
    fn new(
        start: Coord,
        path: Vec<Coord>,
        magazine: VecDeque<CharacterId>,
        visual: CharacterVisual,
    ) -> Self {
        let path = if path.is_empty() { vec![start] } else { path };
        Self {
            path,
            dist: 0.0,
            magazine,
            visual,
        }
    }

    fn max_dist(&self) -> f64 {
        self.path.len().saturating_sub(1) as f64
    }

    fn finished(&self) -> bool {
        self.dist >= self.max_dist()
    }

    fn pos(&self) -> Coord {
        if self.path.is_empty() {
            return Coord::new(1, 1);
        }
        let max_idx = self.path.len() - 1;
        let idx = (self.dist.floor() as usize).min(max_idx);
        if idx >= max_idx {
            return self.path[max_idx];
        }
        let frac = self.dist - idx as f64;
        lerp_coord(self.path[idx], self.path[idx + 1], frac)
    }

    fn step(&mut self) {
        let max_d = self.max_dist();
        self.dist = (self.dist + LAUNCHER_SPEED).min(max_d);
    }
}

struct Flyer {
    id: CharacterId,
    start: Coord,
    end: Coord,
    traveled: f64,
    total: f64,
    done: bool,
}

fn launch_from(
    magazine: &mut VecDeque<CharacterId>,
    origin: Coord,
    count: usize,
    term: &mut Terminal,
    flying: &mut Vec<Flyer>,
) {
    for _ in 0..count {
        let Some(id) = magazine.pop_front() else {
            break;
        };
        let end = term
            .get_character(id)
            .map(|c| c.input_coord)
            .unwrap_or(origin);
        term.set_character_visibility(id, true);
        if let Some(ch) = term.get_character_mut(id) {
            ch.motion.current_coord = origin;
        }
        let total = distance(origin, end);
        flying.push(Flyer {
            id,
            start: origin,
            end,
            traveled: 0.0,
            total,
            done: total == 0.0,
        });
    }
}

fn paint_frame(term: &mut Terminal, top: &Launcher, bot: &Launcher) -> String {
    let mut draws: Vec<(Coord, CharacterVisual)> = term
        .get_characters()
        .iter()
        .filter(|ch| ch.is_visible)
        .map(|ch| {
            let mut visual = ch.animation.current_character_visual.clone();
            if visual.symbol.is_empty() {
                visual.symbol = ch.input_symbol.clone();
                visual.refresh();
            }
            (ch.current_coord(), visual)
        })
        .collect();
    draws.push((top.pos(), top.visual.clone()));
    draws.push((bot.pos(), bot.visual.clone()));
    term.canvas.clear();
    for (coord, visual) in draws {
        term.canvas.put(coord, visual);
    }
    term.canvas.render()
}

fn launcher_visual(symbol: &str) -> CharacterVisual {
    let mut visual = CharacterVisual::new(symbol, None);
    visual.colors = Some(ColorPair::fg(Color::rgb(0xff, 0xff, 0xff)));
    visual.refresh();
    visual
}

fn perimeter_path(start: Coord, canvas: &crate::engine::canvas::Canvas) -> Vec<Coord> {
    let corners = [
        Coord::new(canvas.right, canvas.top),
        Coord::new(canvas.right, canvas.bottom),
        Coord::new(canvas.left, canvas.bottom),
        Coord::new(canvas.left, canvas.top),
    ];
    let start_idx = corners.iter().position(|&c| c == start).unwrap_or(0);
    let mut waypoints = Vec::with_capacity(5);
    for i in 0..4 {
        waypoints.push(corners[(start_idx + i) % 4]);
    }
    waypoints.push(corners[start_idx]);

    let mut path = Vec::new();
    for pair in waypoints.windows(2) {
        let mut line = find_coords_on_line(pair[0], pair[1]);
        if !path.is_empty() && !line.is_empty() {
            line.remove(0);
        }
        path.extend(line);
    }
    if path.is_empty() {
        path.push(start);
    }
    path
}

fn out_sine(t: f64) -> f64 {
    (t.clamp(0.0, 1.0) * std::f64::consts::FRAC_PI_2).sin()
}
