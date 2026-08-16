use std::collections::VecDeque;

use clap::{Arg, ArgAction, ArgMatches, Command};

use super::Effect;

#[derive(Clone, Debug)]
pub struct Pour {
    pour_direction: String,
    pour_speed: usize,
    movement_speed_min: f64,
    movement_speed_max: f64,
    gap: usize,
    starting_color: String,
    movement_easing: String,
    final_gradient_stops: Vec<String>,
    final_gradient_steps: usize,
    final_gradient_frames: usize,
    final_gradient_direction: String,
}

impl Default for Pour {
    fn default() -> Self {
        Self::new()
    }
}

impl Pour {
    pub fn new() -> Self {
        Self {
            pour_direction: String::from("down"),
            pour_speed: 2,
            movement_speed_min: 0.4,
            movement_speed_max: 0.6,
            gap: 1,
            starting_color: String::from("ffffff"),
            movement_easing: String::from("IN_QUAD"),
            final_gradient_stops: vec![
                String::from("8A008A"),
                String::from("00D1FF"),
                String::from("ffffff"),
            ],
            final_gradient_steps: 12,
            final_gradient_frames: 6,
            final_gradient_direction: String::from("vertical"),
        }
    }

    pub fn command() -> Command {
        Command::new("pour")
            .about("Pours the characters into position from the given direction.")
            .arg(
                Arg::new("pour-direction")
                    .long("pour-direction")
                    .help("Direction the text will pour.")
                    .value_parser(["up", "down", "left", "right"])
                    .default_value("down"),
            )
            .arg(
                Arg::new("pour-speed")
                    .long("pour-speed")
                    .help("Number of characters poured in per tick. Increase to speed up the effect.")
                    .value_parser(clap::value_parser!(u64).range(1..))
                    .default_value("2"),
            )
            .arg(
                Arg::new("movement-speed-range")
                    .long("movement-speed-range")
                    .help("Movement speed range of the characters.")
                    .default_value("0.4-0.6"),
            )
            .arg(
                Arg::new("gap")
                    .long("gap")
                    .help(
                        "Number of frames to wait between each character in the pour effect. \
                         Increase to slow down effect and create a more defined back and forth motion.",
                    )
                    .value_parser(clap::value_parser!(u64))
                    .default_value("1"),
            )
            .arg(
                Arg::new("starting-color")
                    .long("starting-color")
                    .help("Color of the characters before the gradient starts.")
                    .default_value("ffffff"),
            )
            .arg(
                Arg::new("movement-easing")
                    .long("movement-easing")
                    .help("Easing function to use for character movement.")
                    .default_value("IN_QUAD"),
            )
            .arg(
                Arg::new("final-gradient-stops")
                    .long("final-gradient-stops")
                    .help("Space separated, unquoted, list of colors for the character gradient.")
                    .num_args(1..)
                    .action(ArgAction::Append)
                    .default_values(["8A008A", "00D1FF", "ffffff"]),
            )
            .arg(
                Arg::new("final-gradient-steps")
                    .long("final-gradient-steps")
                    .help("Number of gradient steps to use.")
                    .value_parser(clap::value_parser!(u64).range(1..))
                    .default_value("12"),
            )
            .arg(
                Arg::new("final-gradient-frames")
                    .long("final-gradient-frames")
                    .help("Number of frames to display each gradient step.")
                    .value_parser(clap::value_parser!(u64).range(1..))
                    .default_value("6"),
            )
            .arg(
                Arg::new("final-gradient-direction")
                    .long("final-gradient-direction")
                    .help("Direction of the final gradient.")
                    .value_parser(["vertical", "horizontal", "diagonal", "radial"])
                    .default_value("vertical"),
            )
    }

    pub fn from_matches(matches: &ArgMatches) -> Self {
        let mut cfg = Self::new();
        if let Some(v) = matches.get_one::<String>("pour-direction") {
            cfg.pour_direction = v.clone();
        }
        if let Some(v) = matches.get_one::<u64>("pour-speed") {
            cfg.pour_speed = (*v as usize).max(1);
        }
        if let Some(v) = matches.get_one::<String>("movement-speed-range") {
            if let Some((a, b)) = parse_float_range(v) {
                cfg.movement_speed_min = a;
                cfg.movement_speed_max = b;
            }
        }
        if let Some(v) = matches.get_one::<u64>("gap") {
            cfg.gap = *v as usize;
        }
        if let Some(v) = matches.get_one::<String>("starting-color") {
            cfg.starting_color = v.clone();
        }
        if let Some(v) = matches.get_one::<String>("movement-easing") {
            cfg.movement_easing = v.clone();
        }
        let stops: Vec<String> = matches
            .get_many::<String>("final-gradient-stops")
            .map(|vals| vals.cloned().collect())
            .unwrap_or_default();
        if !stops.is_empty() {
            cfg.final_gradient_stops = stops;
        }
        if let Some(v) = matches.get_one::<u64>("final-gradient-steps") {
            cfg.final_gradient_steps = (*v as usize).max(1);
        }
        if let Some(v) = matches.get_one::<u64>("final-gradient-frames") {
            cfg.final_gradient_frames = (*v as usize).max(1);
        }
        if let Some(v) = matches.get_one::<String>("final-gradient-direction") {
            cfg.final_gradient_direction = v.clone();
        }
        cfg
    }
}

impl Effect for Pour {
    fn name(&self) -> &str {
        "pour"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let grid_src = parse_input(input);
        let rows = grid_src.len();
        let cols = grid_src.iter().map(|row| row.len()).max().unwrap_or(0);
        if rows == 0 || cols == 0 {
            return Vec::new();
        }

        let speed = ((self.movement_speed_min + self.movement_speed_max) * 0.5).max(0.0001);
        let start_color = parse_hex(&self.starting_color);
        let stops: Vec<(u8, u8, u8)> = self
            .final_gradient_stops
            .iter()
            .map(|s| parse_hex(s))
            .collect();

        let mut poured: Vec<Poured> = Vec::new();
        for (r, row) in grid_src.iter().enumerate() {
            for (c, &ch) in row.iter().enumerate() {
                let (sr, sc) = start_coord(&self.pour_direction, r, c, rows, cols);
                let dest_color = gradient_color(
                    &stops,
                    &self.final_gradient_direction,
                    r,
                    c,
                    rows,
                    cols,
                    self.final_gradient_steps.max(1),
                );
                let dist = ((r as f64 - sr).powi(2) + (c as f64 - sc).powi(2)).sqrt();
                poured.push(Poured {
                    ch,
                    dest_r: r,
                    dest_c: c,
                    start_r: sr,
                    start_c: sc,
                    r: sr,
                    c: sc,
                    dist,
                    progress: 0.0,
                    visible: false,
                    done: dist == 0.0,
                    color_tick: 0,
                    dest_color,
                });
            }
        }

        if poured.is_empty() {
            return Vec::new();
        }

        let mut groups = build_groups(&self.pour_direction, rows, cols, &grid_src);
        for (i, group) in groups.iter_mut().enumerate() {
            if i % 2 == 1 {
                group.reverse();
            }
        }

        let mut pending: VecDeque<VecDeque<usize>> = groups.into_iter().map(VecDeque::from).collect();
        let mut current: VecDeque<usize> = VecDeque::new();
        let mut gap = 0usize;
        let mut frames = Vec::new();
        let color_span = self
            .final_gradient_steps
            .max(1)
            .saturating_mul(self.final_gradient_frames.max(1)) as f64;

        for _ in 0..100_000 {
            let any_active = poured.iter().any(|p| p.visible && !p.done);
            if pending.is_empty() && current.is_empty() && !any_active {
                break;
            }

            if current.is_empty() {
                if let Some(next) = pending.pop_front() {
                    current = next;
                }
            }

            if !current.is_empty() {
                if gap == 0 {
                    for _ in 0..self.pour_speed.max(1) {
                        if let Some(idx) = current.pop_front() {
                            poured[idx].visible = true;
                        }
                    }
                    gap = self.gap;
                } else {
                    gap -= 1;
                }
            }

            for p in poured.iter_mut() {
                if !p.visible || p.done {
                    continue;
                }
                p.color_tick = p.color_tick.saturating_add(1);
                if p.dist <= 0.0 {
                    p.r = p.dest_r as f64;
                    p.c = p.dest_c as f64;
                    p.done = true;
                    continue;
                }
                p.progress = (p.progress + speed / p.dist).min(1.0);
                let t = ease(&self.movement_easing, p.progress);
                p.r = p.start_r + (p.dest_r as f64 - p.start_r) * t;
                p.c = p.start_c + (p.dest_c as f64 - p.start_c) * t;
                if p.progress >= 1.0 {
                    p.r = p.dest_r as f64;
                    p.c = p.dest_c as f64;
                    p.done = true;
                }
            }

            frames.push(render_frame(&poured, rows, cols, start_color, color_span));
        }

        frames
    }
}

struct Poured {
    ch: char,
    dest_r: usize,
    dest_c: usize,
    start_r: f64,
    start_c: f64,
    r: f64,
    c: f64,
    dist: f64,
    progress: f64,
    visible: bool,
    done: bool,
    color_tick: usize,
    dest_color: (u8, u8, u8),
}

fn parse_input(input: &str) -> Vec<Vec<char>> {
    let mut rows = Vec::new();
    for raw in input.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let mut row = Vec::new();
        for ch in line.chars() {
            if ch == '\t' {
                let pad = 4 - (row.len() % 4);
                row.extend(std::iter::repeat(' ').take(pad));
            } else {
                row.push(ch);
            }
        }
        rows.push(row);
    }
    if input.ends_with('\n') && rows.last().is_some_and(|r| r.is_empty()) {
        rows.pop();
    }
    rows
}

fn start_coord(direction: &str, r: usize, c: usize, rows: usize, cols: usize) -> (f64, f64) {
    match direction {
        "up" => ((rows.saturating_sub(1)) as f64, c as f64),
        "left" => (r as f64, (cols.saturating_sub(1)) as f64),
        "right" => (r as f64, 0.0),
        _ => (0.0, c as f64),
    }
}

fn build_groups(
    direction: &str,
    rows: usize,
    cols: usize,
    grid: &[Vec<char>],
) -> Vec<Vec<usize>> {
    let index = |r: usize, c: usize| -> Option<usize> {
        if r >= rows {
            return None;
        }
        if c >= grid[r].len() {
            return None;
        }
        let mut idx = 0usize;
        for rr in 0..r {
            idx += grid[rr].len();
        }
        Some(idx + c)
    };

    let mut groups = Vec::new();
    match direction {
        "up" => {
            for r in (0..rows).rev() {
                let mut group = Vec::new();
                for c in 0..cols {
                    if let Some(i) = index(r, c) {
                        group.push(i);
                    }
                }
                if !group.is_empty() {
                    groups.push(group);
                }
            }
        }
        "left" => {
            for c in 0..cols {
                let mut group = Vec::new();
                for r in 0..rows {
                    if let Some(i) = index(r, c) {
                        group.push(i);
                    }
                }
                if !group.is_empty() {
                    groups.push(group);
                }
            }
        }
        "right" => {
            for c in (0..cols).rev() {
                let mut group = Vec::new();
                for r in 0..rows {
                    if let Some(i) = index(r, c) {
                        group.push(i);
                    }
                }
                if !group.is_empty() {
                    groups.push(group);
                }
            }
        }
        _ => {
            for r in 0..rows {
                let mut group = Vec::new();
                for c in 0..cols {
                    if let Some(i) = index(r, c) {
                        group.push(i);
                    }
                }
                if !group.is_empty() {
                    groups.push(group);
                }
            }
        }
    }
    groups
}

fn render_frame(
    poured: &[Poured],
    rows: usize,
    cols: usize,
    start_color: (u8, u8, u8),
    color_span: f64,
) -> String {
    let mut grid: Vec<Vec<Option<(char, (u8, u8, u8))>>> = vec![vec![None; cols]; rows];
    for p in poured {
        if !p.visible {
            continue;
        }
        let rr = p.r.round() as i64;
        let cc = p.c.round() as i64;
        if rr < 0 || cc < 0 {
            continue;
        }
        let rr = rr as usize;
        let cc = cc as usize;
        if rr >= rows || cc >= cols {
            continue;
        }
        let t = if color_span <= 0.0 {
            1.0
        } else {
            (p.color_tick as f64 / color_span).clamp(0.0, 1.0)
        };
        let color = if p.done {
            p.dest_color
        } else {
            lerp_rgb(start_color, p.dest_color, t)
        };
        grid[rr][cc] = Some((p.ch, color));
    }

    let mut out = String::new();
    for (ri, row) in grid.iter().enumerate() {
        if ri > 0 {
            out.push('\n');
        }
        for cell in row {
            match cell {
                Some((ch, (r, g, b))) => {
                    out.push_str(&format!("\x1b[38;2;{r};{g};{b}m{ch}\x1b[0m"));
                }
                None => out.push(' '),
            }
        }
    }
    out
}

fn parse_float_range(raw: &str) -> Option<(f64, f64)> {
    let mut parts = raw.split('-');
    let a = parts.next()?.parse::<f64>().ok()?;
    let b = parts.next()?.parse::<f64>().ok()?;
    Some((a, b))
}

fn parse_hex(raw: &str) -> (u8, u8, u8) {
    let s = raw.trim().trim_start_matches('#');
    if s.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&s[0..2], 16),
            u8::from_str_radix(&s[2..4], 16),
            u8::from_str_radix(&s[4..6], 16),
        ) {
            return (r, g, b);
        }
    }
    if s.len() == 3 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&s[0..1], 16),
            u8::from_str_radix(&s[1..2], 16),
            u8::from_str_radix(&s[2..3], 16),
        ) {
            return (r * 17, g * 17, b * 17);
        }
    }
    if let Ok(n) = s.parse::<u16>() {
        if n <= 255 {
            let v = n as u8;
            return (v, v, v);
        }
    }
    (255, 255, 255)
}

fn lerp_rgb(a: (u8, u8, u8), b: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    (
        (a.0 as f64 + (b.0 as f64 - a.0 as f64) * t).round() as u8,
        (a.1 as f64 + (b.1 as f64 - a.1 as f64) * t).round() as u8,
        (a.2 as f64 + (b.2 as f64 - a.2 as f64) * t).round() as u8,
    )
}

fn lerp_stops(stops: &[(u8, u8, u8)], t: f64) -> (u8, u8, u8) {
    if stops.is_empty() {
        return (255, 255, 255);
    }
    if stops.len() == 1 {
        return stops[0];
    }
    let t = t.clamp(0.0, 1.0) * (stops.len() - 1) as f64;
    let i = t.floor() as usize;
    let i = i.min(stops.len() - 2);
    let frac = t - i as f64;
    lerp_rgb(stops[i], stops[i + 1], frac)
}

fn gradient_color(
    stops: &[(u8, u8, u8)],
    direction: &str,
    r: usize,
    c: usize,
    rows: usize,
    cols: usize,
    steps: usize,
) -> (u8, u8, u8) {
    let rt = if rows <= 1 {
        0.0
    } else {
        r as f64 / (rows - 1) as f64
    };
    let ct = if cols <= 1 {
        0.0
    } else {
        c as f64 / (cols - 1) as f64
    };
    let mut t = match direction {
        "horizontal" => ct,
        "diagonal" => (rt + ct) * 0.5,
        "radial" => {
            let dr = rt - 0.5;
            let dc = ct - 0.5;
            ((dr * dr + dc * dc).sqrt() * 2.0).clamp(0.0, 1.0)
        }
        _ => rt,
    };
    let steps = steps.max(1) as f64;
    t = (t * steps).floor() / steps;
    lerp_stops(stops, t)
}

fn ease(name: &str, t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    match name.to_ascii_uppercase().replace('-', "_").as_str() {
        "LINEAR" => t,
        "IN_QUAD" => t * t,
        "OUT_QUAD" => 1.0 - (1.0 - t) * (1.0 - t),
        "IN_OUT_QUAD" => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }
        "IN_CUBIC" => t * t * t,
        "OUT_CUBIC" => 1.0 - (1.0 - t).powi(3),
        "IN_OUT_CUBIC" => {
            if t < 0.5 {
                4.0 * t * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
            }
        }
        "IN_QUART" => t.powi(4),
        "OUT_QUART" => 1.0 - (1.0 - t).powi(4),
        "IN_SINE" => 1.0 - (t * std::f64::consts::FRAC_PI_2).cos(),
        "OUT_SINE" => (t * std::f64::consts::FRAC_PI_2).sin(),
        _ => t * t,
    }
}
