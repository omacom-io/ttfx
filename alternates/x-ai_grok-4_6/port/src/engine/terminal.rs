use std::collections::HashMap;
use std::io::{self, Write};

use crossterm::{cursor, execute, terminal as cterm};

use crate::engine::animation::CharacterVisual;
use crate::engine::canvas::Canvas;
use crate::engine::character::{CharacterId, EffectCharacter};
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Anchor {
    #[default]
    Center,
    N,
    Ne,
    E,
    Se,
    S,
    Sw,
    W,
    Nw,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExistingColorHandling {
    Always,
    Dynamic,
    #[default]
    Ignore,
}

#[derive(Clone, Debug)]
pub struct TerminalConfig {
    pub tab_width: usize,
    pub wrap_text: bool,
    pub frame_rate: f64,
    pub canvas_width: Option<usize>,
    pub canvas_height: Option<usize>,
    pub anchor_canvas: Anchor,
    pub anchor_text: Anchor,
    pub ignore_terminal_dimensions: bool,
    pub existing_color_handling: ExistingColorHandling,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            tab_width: 4,
            wrap_text: false,
            frame_rate: 60.0,
            canvas_width: None,
            canvas_height: None,
            anchor_canvas: Anchor::Center,
            anchor_text: Anchor::Center,
            ignore_terminal_dimensions: false,
            existing_color_handling: ExistingColorHandling::Ignore,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Terminal {
    pub config: TerminalConfig,
    pub canvas: Canvas,
    characters: Vec<EffectCharacter>,
}

impl Terminal {
    pub fn from_input(input: &str, config: TerminalConfig) -> Self {
        let tab_width = config.tab_width.max(1);
        let parsed = parse_ansi_text(input, tab_width);
        let text_w = parsed.iter().map(Vec::len).max().unwrap_or(0);
        let text_h = parsed.len();

        let (term_w, term_h) = cterm::size().unwrap_or((80, 24));
        let (width, height) = if config.ignore_terminal_dimensions {
            (
                config.canvas_width.unwrap_or(text_w.max(1)),
                config.canvas_height.unwrap_or(text_h.max(1)),
            )
        } else {
            (
                config.canvas_width.unwrap_or(term_w as usize).max(1),
                config.canvas_height.unwrap_or(term_h as usize).max(1),
            )
        };

        let mut lines = parsed;
        if config.wrap_text && width > 0 {
            lines = wrap_parsed_lines(lines, width);
        }

        let text_w = lines.iter().map(Vec::len).max().unwrap_or(0);
        let text_h = lines.len();
        let canvas = Canvas::new(width, height);
        let origin = text_origin(&canvas, text_w, text_h, config.anchor_text);

        let mut characters = Vec::new();
        let mut next_id = 0u32;
        for (line_idx, line) in lines.iter().enumerate() {
            let row = origin.row + (text_h as i32 - 1 - line_idx as i32);
            for (col_idx, cell) in line.iter().enumerate() {
                let coord = Coord {
                    column: origin.column + col_idx as i32,
                    row,
                };
                if !canvas.contains(coord) {
                    continue;
                }
                let id = CharacterId(next_id);
                next_id += 1;
                let mut ch = EffectCharacter::new(id, cell.ch.to_string(), coord);
                ch.input_fg = cell.style.fg;
                ch.input_bg = cell.style.bg;
                match config.existing_color_handling {
                    ExistingColorHandling::Always | ExistingColorHandling::Dynamic => {
                        let pair = ColorPair {
                            fg: cell.style.fg,
                            bg: cell.style.bg,
                        };
                        if pair.fg.is_some() || pair.bg.is_some() {
                            ch.animation.set_appearance(&ch.input_symbol, Some(pair));
                        }
                        let visual = ch.animation.current_character_visual.clone();
                        ch.animation.current_character_visual.bold = cell.style.bold;
                        ch.animation.current_character_visual.dim = cell.style.dim;
                        ch.animation.current_character_visual.italic = cell.style.italic;
                        ch.animation.current_character_visual.underline = cell.style.underline;
                        ch.animation.current_character_visual.blink = cell.style.blink;
                        ch.animation.current_character_visual.reverse = cell.style.reverse;
                        ch.animation.current_character_visual.hidden = cell.style.hidden;
                        ch.animation.current_character_visual.strike = cell.style.strike;
                        if visual.colors.is_none() {
                            ch.animation.current_character_visual.colors = pair.into_option();
                        }
                        ch.animation.current_character_visual.refresh();
                    }
                    ExistingColorHandling::Ignore => {}
                }
                characters.push(ch);
            }
        }

        link_neighbors(&mut characters);

        Self {
            config,
            canvas,
            characters,
        }
    }

    pub fn get_characters(&self) -> &[EffectCharacter] {
        &self.characters
    }

    pub fn get_characters_mut(&mut self) -> &mut [EffectCharacter] {
        &mut self.characters
    }

    pub fn get_character(&self, id: CharacterId) -> Option<&EffectCharacter> {
        self.characters.iter().find(|ch| ch.id == id)
    }

    pub fn get_character_mut(&mut self, id: CharacterId) -> Option<&mut EffectCharacter> {
        self.characters.iter_mut().find(|ch| ch.id == id)
    }

    pub fn set_character_visibility(&mut self, id: CharacterId, is_visible: bool) {
        if let Some(ch) = self.get_character_mut(id) {
            ch.is_visible = is_visible;
        }
    }

    pub fn show_all(&mut self) {
        for ch in &mut self.characters {
            ch.is_visible = true;
        }
    }

    pub fn hide_all(&mut self) {
        for ch in &mut self.characters {
            ch.is_visible = false;
        }
    }

    pub fn tick(&mut self) {
        for ch in &mut self.characters {
            if ch.is_visible || ch.is_active() {
                ch.tick();
            }
        }
    }

    pub fn render_frame(&mut self) -> String {
        self.canvas.clear();
        for ch in &self.characters {
            if !ch.is_visible {
                continue;
            }
            let mut visual = ch.animation.current_character_visual.clone();
            if visual.symbol.is_empty() {
                visual.symbol = ch.input_symbol.clone();
                visual.refresh();
            }
            self.canvas.put(ch.current_coord(), visual);
        }
        self.canvas.render()
    }

    pub fn get_formatted_output_string(&mut self) -> String {
        self.render_frame()
    }

    pub fn get_next_frame(&mut self) -> String {
        self.render_frame()
    }

    pub fn character_count(&self) -> usize {
        self.characters.len()
    }
}

pub struct TtyWriter;

impl TtyWriter {
    pub fn prep() -> io::Result<()> {
        let mut out = io::stdout();
        execute!(out, cursor::Hide, cterm::EnterAlternateScreen)?;
        out.flush()
    }

    pub fn restore() -> io::Result<()> {
        let mut out = io::stdout();
        execute!(out, cursor::Show, cterm::LeaveAlternateScreen)?;
        out.flush()
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct StyleState {
    fg: Option<Color>,
    bg: Option<Color>,
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    blink: bool,
    reverse: bool,
    hidden: bool,
    strike: bool,
}

impl StyleState {
    fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Clone, Copy, Debug)]
struct ParsedCell {
    ch: char,
    style: StyleState,
}

fn parse_ansi_text(input: &str, tab_width: usize) -> Vec<Vec<ParsedCell>> {
    let mut lines: Vec<Vec<ParsedCell>> = vec![Vec::new()];
    let mut style = StyleState::default();
    let mut chars = input.chars().peekable();
    enum Mode {
        Ground,
        Esc,
        Csi,
        Osc,
        OscSt,
    }
    let mut mode = Mode::Ground;
    let mut csi = String::new();

    while let Some(ch) = chars.next() {
        match mode {
            Mode::Ground => match ch {
                '\u{1b}' => mode = Mode::Esc,
                '\n' => lines.push(Vec::new()),
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    lines.push(Vec::new());
                }
                '\t' => {
                    let col = lines.last().map(Vec::len).unwrap_or(0);
                    let spaces = tab_width - (col % tab_width);
                    if let Some(line) = lines.last_mut() {
                        for _ in 0..spaces {
                            line.push(ParsedCell { ch: ' ', style });
                        }
                    }
                }
                c if (c as u32) < 32 => {}
                c => {
                    if let Some(line) = lines.last_mut() {
                        line.push(ParsedCell { ch: c, style });
                    }
                }
            },
            Mode::Esc => match ch {
                '[' => {
                    csi.clear();
                    mode = Mode::Csi;
                }
                ']' => mode = Mode::Osc,
                _ => mode = Mode::Ground,
            },
            Mode::Csi => {
                if ('@'..='~').contains(&ch) {
                    if ch == 'm' {
                        apply_sgr(&mut style, &parse_csi_params(&csi));
                    }
                    mode = Mode::Ground;
                } else {
                    csi.push(ch);
                }
            }
            Mode::Osc => match ch {
                '\u{07}' => mode = Mode::Ground,
                '\u{1b}' => mode = Mode::OscSt,
                _ => {}
            },
            Mode::OscSt => {
                mode = if ch == '\\' { Mode::Ground } else { Mode::Osc };
            }
        }
    }

    if lines.last().is_some_and(|l| l.is_empty()) && lines.len() > 1 {
        lines.pop();
    }
    lines
}

fn parse_csi_params(raw: &str) -> Vec<i32> {
    let body = raw.trim_start_matches(['?', '>', '<', '=']);
    if body.is_empty() {
        return vec![0];
    }
    body.split([';', ':'])
        .map(|p| p.parse().unwrap_or(0))
        .collect()
}

fn apply_sgr(state: &mut StyleState, params: &[i32]) {
    if params.is_empty() {
        state.reset();
        return;
    }
    let mut i = 0;
    while i < params.len() {
        match params[i] {
            0 => state.reset(),
            1 => state.bold = true,
            2 => state.dim = true,
            3 => state.italic = true,
            4 => state.underline = true,
            5 | 6 => state.blink = true,
            7 => state.reverse = true,
            8 => state.hidden = true,
            9 => state.strike = true,
            22 => {
                state.bold = false;
                state.dim = false;
            }
            23 => state.italic = false,
            24 => state.underline = false,
            25 => state.blink = false,
            27 => state.reverse = false,
            28 => state.hidden = false,
            29 => state.strike = false,
            39 => state.fg = None,
            49 => state.bg = None,
            n @ 30..=37 => state.fg = Some(Color::from_xterm((n - 30) as u8)),
            n @ 90..=97 => state.fg = Some(Color::from_xterm((n - 90 + 8) as u8)),
            n @ 40..=47 => state.bg = Some(Color::from_xterm((n - 40) as u8)),
            n @ 100..=107 => state.bg = Some(Color::from_xterm((n - 100 + 8) as u8)),
            38 | 48 => {
                let is_fg = params[i] == 38;
                if i + 2 < params.len() && params[i + 1] == 5 {
                    let c = Color::from_xterm(params[i + 2].clamp(0, 255) as u8);
                    if is_fg {
                        state.fg = Some(c);
                    } else {
                        state.bg = Some(c);
                    }
                    i += 2;
                } else if i + 4 < params.len() && params[i + 1] == 2 {
                    let c = Color::rgb(
                        params[i + 2].clamp(0, 255) as u8,
                        params[i + 3].clamp(0, 255) as u8,
                        params[i + 4].clamp(0, 255) as u8,
                    );
                    if is_fg {
                        state.fg = Some(c);
                    } else {
                        state.bg = Some(c);
                    }
                    i += 4;
                }
            }
            _ => {}
        }
        i += 1;
    }
}

fn wrap_parsed_lines(lines: Vec<Vec<ParsedCell>>, width: usize) -> Vec<Vec<ParsedCell>> {
    if width == 0 {
        return lines;
    }
    let mut out = Vec::new();
    for line in lines {
        if line.is_empty() {
            out.push(Vec::new());
            continue;
        }
        for chunk in line.chunks(width) {
            out.push(chunk.to_vec());
        }
    }
    out
}

fn text_origin(canvas: &Canvas, text_w: usize, text_h: usize, anchor: Anchor) -> Coord {
    let cw = canvas.width as i32;
    let ch = canvas.height as i32;
    let tw = text_w as i32;
    let th = text_h as i32;
    let (dx, dy) = match anchor {
        Anchor::Center => ((cw - tw) / 2, (ch - th) / 2),
        Anchor::N => ((cw - tw) / 2, ch - th),
        Anchor::S => ((cw - tw) / 2, 0),
        Anchor::E => (cw - tw, (ch - th) / 2),
        Anchor::W => (0, (ch - th) / 2),
        Anchor::Ne => (cw - tw, ch - th),
        Anchor::Nw => (0, ch - th),
        Anchor::Se => (cw - tw, 0),
        Anchor::Sw => (0, 0),
    };
    Coord {
        column: canvas.left + dx,
        row: canvas.bottom + dy,
    }
}

fn link_neighbors(characters: &mut [EffectCharacter]) {
    let mut by_coord: HashMap<Coord, CharacterId> = HashMap::new();
    for ch in characters.iter() {
        by_coord.insert(ch.input_coord, ch.id);
    }
    for ch in characters.iter_mut() {
        let c = ch.input_coord;
        ch.neighbors.left = by_coord
            .get(&Coord {
                column: c.column - 1,
                row: c.row,
            })
            .copied();
        ch.neighbors.right = by_coord
            .get(&Coord {
                column: c.column + 1,
                row: c.row,
            })
            .copied();
        ch.neighbors.above = by_coord
            .get(&Coord {
                column: c.column,
                row: c.row + 1,
            })
            .copied();
        ch.neighbors.below = by_coord
            .get(&Coord {
                column: c.column,
                row: c.row - 1,
            })
            .copied();
    }
}
