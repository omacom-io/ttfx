use super::Effect;
use crate::engine::canvas::Cell;
use crate::engine::terminal::Terminal;
use crate::utils::easing::{self, EasingFn};
use crate::utils::geometry::Coord;

pub struct Slide {
    grouping: &'static str,
    merge: bool,
    reverse_direction: bool,
    movement_easing: EasingFn,
}

impl Slide {
    pub fn new() -> Self {
        Self {
            grouping: "row",
            merge: false,
            reverse_direction: false,
            movement_easing: easing::ease_out_quad,
        }
    }

    fn build_moves(&self, terminal: &Terminal) -> Vec<CharacterMove> {
        let mut groups: Vec<Vec<usize>> = Vec::new();

        if self.grouping == "column" {
            for x in 0..terminal.canvas.width {
                let mut group: Vec<usize> = Vec::new();
                for (idx, ch) in terminal.characters.iter().enumerate() {
                    if ch.position.x as u16 == x {
                        group.push(idx);
                    }
                }
                if !group.is_empty() {
                    group.sort_by_key(|&i| terminal.characters[i].position.y as i32);
                    groups.push(group);
                }
            }
        } else {
            for y in 0..terminal.canvas.height {
                let mut group: Vec<usize> = Vec::new();
                for (idx, ch) in terminal.characters.iter().enumerate() {
                    if ch.position.y as u16 == y {
                        group.push(idx);
                    }
                }
                if !group.is_empty() {
                    group.sort_by_key(|&i| terminal.characters[i].position.x as i32);
                    groups.push(group);
                }
            }
        }

        let mut moves = Vec::new();

        for (group_index, group) in groups.iter().enumerate() {
            let mut group = group.clone();

            if self.grouping == "column" {
                let mut from_bottom = false;

                if self.merge && group_index % 2 == 0 {
                    from_bottom = true;
                } else {
                    group.reverse();
                }

                if self.reverse_direction && !self.merge {
                    group.reverse();
                }

                let min_y = group
                    .iter()
                    .map(|&i| terminal.characters[i].position.y)
                    .fold(f32::MAX, f32::min);
                let max_y = group
                    .iter()
                    .map(|&i| terminal.characters[i].position.y)
                    .fold(f32::MIN, f32::max);

                let offset = if from_bottom {
                    terminal.canvas.height as f32 - min_y
                } else {
                    -(max_y + 1.0)
                };

                for &idx in &group {
                    let end = terminal.characters[idx].position;
                    moves.push(CharacterMove {
                        char_index: idx,
                        start: Coord::new(end.x, end.y + offset),
                        end,
                    });
                }
            } else {
                let mut from_right = false;

                if self.merge && group_index % 2 == 0 {
                    from_right = true;
                } else {
                    group.reverse();
                }

                if self.reverse_direction && !self.merge {
                    group.reverse();
                }

                let min_x = group
                    .iter()
                    .map(|&i| terminal.characters[i].position.x)
                    .fold(f32::MAX, f32::min);
                let max_x = group
                    .iter()
                    .map(|&i| terminal.characters[i].position.x)
                    .fold(f32::MIN, f32::max);

                let offset = if from_right {
                    terminal.canvas.width as f32 - min_x
                } else {
                    -(max_x + 1.0)
                };

                for &idx in &group {
                    let end = terminal.characters[idx].position;
                    moves.push(CharacterMove {
                        char_index: idx,
                        start: Coord::new(end.x + offset, end.y),
                        end,
                    });
                }
            }
        }

        moves
    }
}

impl Effect for Slide {
    fn name(&self) -> &str {
        "slide"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (width, height) = Terminal::autodetect_size();
        let terminal = Terminal::from_input(input, width, height);
        let moves = self.build_moves(&terminal);

        if moves.is_empty() {
            return vec![terminal.write_frame()];
        }

        let total_frames = 20usize;
        let mut frames = Vec::with_capacity(total_frames + 1);

        for step in 0..=total_frames {
            let t = (self.movement_easing)(step as f32 / total_frames as f32);
            let mut canvas = terminal.canvas.clone();
            canvas.clear();

            for character_move in &moves {
                let current = character_move.start.lerp(character_move.end, t);
                let x = current.x.round() as i32;
                let y = current.y.round() as i32;

                if x >= 0 && y >= 0 {
                    let x = x as u16;
                    let y = y as u16;

                    if x < canvas.width && y < canvas.height {
                        let character = &terminal.characters[character_move.char_index];
                        canvas.set_cell(
                            x,
                            y,
                            Cell::new(character.output_symbol.clone(), character.style),
                        );
                    }
                }
            }

            frames.push(canvas.render_frame());
        }

        frames
    }
}

struct CharacterMove {
    char_index: usize,
    start: Coord,
    end: Coord,
}
