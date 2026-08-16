use crate::engine::canvas::{Cell, CellStyle};
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, ColorPair};

use super::Effect;

pub struct Binarypath;

impl Binarypath {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Binarypath {
    fn name(&self) -> &str {
        "binarypath"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (width, height) = dimensions_for(input);
        let mut terminal = Terminal::from_input(input, width, height);

        struct CharacterPath {
            symbol: String,
            path: Vec<(i32, i32)>,
        }

        let mut paths = Vec::new();
        for character in &terminal.characters {
            let target_x = character.position.x.round() as i32;
            let target_y = character.position.y.round() as i32;

            let start_x = if character.id % 2 == 0 {
                0
            } else {
                width as i32 - 1
            };
            let start_y = if (character.id / 2) % 2 == 0 {
                0
            } else {
                height as i32 - 1
            };

            let path = binary_path((start_x, start_y), (target_x, target_y));
            paths.push(CharacterPath {
                symbol: character.input_symbol.clone(),
                path,
            });
        }

        let max_len = paths.iter().map(|p| p.path.len()).max().unwrap_or(0);
        if max_len == 0 {
            return vec![terminal.write_frame()];
        }

        let style = CellStyle::with_color_pair(ColorPair::new(Color::GREEN, Color::BLACK));
        let mut frames = Vec::with_capacity(max_len + 5);

        for frame_idx in 0..max_len {
            terminal.clear_canvas();

            for character_path in &paths {
                let idx = frame_idx.min(character_path.path.len() - 1);
                let (x, y) = character_path.path[idx];

                if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                    continue;
                }

                terminal.canvas.set_cell(
                    x as u16,
                    y as u16,
                    Cell::new(character_path.symbol.clone(), style),
                );
            }

            frames.push(terminal.write_frame());
        }

        for _ in 0..5 {
            if let Some(last) = frames.last() {
                frames.push(last.clone());
            }
        }

        frames
    }
}

fn dimensions_for(input: &str) -> (u16, u16) {
    let line_count = input.lines().count().max(1);
    let max_line = input
        .lines()
        .map(|line| line.chars().count().max(1))
        .max()
        .unwrap_or(1);

    let width = (max_line as u32 + 2).min(200) as u16;
    let height = (line_count as u32 + 2).min(100) as u16;

    (width.max(5), height.max(5))
}

fn binary_path(start: (i32, i32), end: (i32, i32)) -> Vec<(i32, i32)> {
    if start == end {
        return vec![start];
    }

    let mut waypoints = vec![start];
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let adx = dx.abs();
    let ady = dy.abs();

    let mut bit = 1;
    while bit <= adx || bit <= ady {
        bit <<= 1;
    }
    bit >>= 1;

    let mut current = start;
    while bit > 0 {
        if adx & bit != 0 {
            current.0 += if dx >= 0 { bit } else { -bit };
            waypoints.push(current);
        }
        if ady & bit != 0 {
            current.1 += if dy >= 0 { bit } else { -bit };
            waypoints.push(current);
        }
        bit >>= 1;
    }

    if *waypoints.last().unwrap() != end {
        waypoints.push(end);
    }

    let mut path = Vec::new();
    for pair in waypoints.windows(2) {
        let from = pair[0];
        let to = pair[1];
        let mut cur = from;
        path.push(cur);

        while cur != to {
            if cur.0 != to.0 {
                cur.0 += if to.0 > cur.0 { 1 } else { -1 };
            } else if cur.1 != to.1 {
                cur.1 += if to.1 > cur.1 { 1 } else { -1 };
            }
            path.push(cur);
        }
    }

    if path.first() != Some(&start) {
        path.insert(0, start);
    }
    if path.last() != Some(&end) {
        path.push(end);
    }

    path
}
