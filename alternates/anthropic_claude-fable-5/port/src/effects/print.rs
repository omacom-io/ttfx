//! Print effect: lines are "printed" one at a time at the bottom of the canvas
//! by a typing head that sweeps left-to-right, performs an eased carriage
//! return between rows, and feeds previously printed rows upward like paper
//! through a line printer. Each printed character resolves through the block
//! symbols █ ▓ ▒ ░ while fading from the print-head color to its final
//! gradient color.
//!
//! Port of terminaltexteffects/effects/effect_print.py.

use super::Effect;
use crate::engine::character::EffectCharacter;
use crate::engine::terminal::{Terminal, TerminalConfig};
use crate::utils::easing;
use crate::utils::geometry::Coord;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Symbols a character passes through while being "printed".
const TYPED_SYMBOLS: [char; 4] = ['█', '▓', '▒', '░'];

/// Safety cap so a malformed state machine can never loop forever.
const MAX_FRAMES: usize = 10_000;

pub struct Print {
    final_gradient_stops: Vec<Color>,
    final_gradient_steps: usize,
    print_head_color: Color,
    /// Characters printed per frame.
    print_speed: usize,
    /// Speed of the carriage-return path.
    print_head_return_speed: f64,
}

impl Print {
    pub fn new() -> Self {
        Print {
            final_gradient_stops: vec![
                Color::from_hex("02b8bd").expect("valid hex"),
                Color::from_hex("c1f0e3").expect("valid hex"),
                Color::from_hex("ffffff").expect("valid hex"),
            ],
            final_gradient_steps: 12,
            print_head_color: Color::from_hex("f3b462").expect("valid hex"),
            print_speed: 1,
            print_head_return_speed: 1.25,
        }
    }
}

impl Default for Print {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Print {
    fn name(&self) -> &str {
        "print"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input, TerminalConfig::default());
        let width = terminal.canvas.width;
        let height = terminal.canvas.height;
        let final_gradient = Gradient::new(&self.final_gradient_stops, self.final_gradient_steps);

        // --- group character indices into rows, top row first, left-to-right.
        // Empty canvas rows are kept so blank input lines still produce a
        // carriage return / line feed and final positions match the input.
        let mut remaining_rows: Vec<Vec<usize>> = Vec::new();
        for row in (1..=height as i32).rev() {
            let mut indices: Vec<usize> = terminal
                .characters
                .iter()
                .enumerate()
                .filter(|(_, c)| c.input_coord.row == row)
                .map(|(i, _)| i)
                .collect();
            indices.sort_by_key(|&i| terminal.characters[i].input_coord.column);
            indices.reverse(); // so pop() yields the leftmost character
            remaining_rows.push(indices);
        }
        remaining_rows.reverse(); // so pop() yields the top row first

        // --- build the "typed" reveal scene for every input character
        // (diagonal final gradient across the canvas, as upstream).
        let denom = (((height as i32) - 1) + ((width as i32) - 1)).max(1) as f64;
        for idx in 0..terminal.characters.len() {
            let (coord, symbol) = {
                let c = &terminal.characters[idx];
                (c.input_coord, c.input_symbol)
            };
            let fraction =
                (((coord.row - 1) + (coord.column - 1)) as f64 / denom).clamp(0.0, 1.0);
            let final_color = final_gradient
                .get_color_at_fraction(fraction)
                .unwrap_or(self.print_head_color);
            // 5 colors: print head color fading into the final color.
            let typed_gradient = Gradient::new(&[self.print_head_color, final_color], 4);
            let character = &mut terminal.characters[idx];
            let scene = character.animation.new_scene("typed", false);
            for (step, color) in typed_gradient.spectrum.iter().enumerate() {
                let sym = *TYPED_SYMBOLS.get(step).unwrap_or(&symbol);
                scene.add_frame(sym, 5, ColorPair::fg(*color), false);
            }
        }

        // --- add the typing head as an extra character parked at bottom-left.
        let head_idx = terminal.characters.len();
        let head_id = head_idx; // ids are allocated 0..n, so this is unique
        terminal
            .characters
            .push(EffectCharacter::new(head_id, '█', Coord::new(1, 1)));
        {
            let head = &mut terminal.characters[head_idx];
            let scene = head.animation.new_scene("head", false);
            scene.add_frame('█', 1, ColorPair::fg(self.print_head_color), false);
            head.animation.activate_scene("head");
        }

        // --- run the effect.
        let print_speed = self.print_speed.max(1);
        let mut current_row: Vec<usize> = remaining_rows.pop().unwrap_or_default();
        let mut typed: Vec<usize> = Vec::new();
        let mut returning = false;
        let mut head_retired = false;
        let mut frames: Vec<String> = Vec::new();

        terminal.set_character_visibility(head_id, true);

        while frames.len() < MAX_FRAMES {
            // finish a carriage return: feed the paper up one row, then load
            // the next pending row.
            if returning && terminal.characters[head_idx].motion.movement_is_complete() {
                returning = false;
                for &idx in &typed {
                    let coord = terminal.characters[idx].motion.current_coord;
                    terminal.characters[idx].motion.current_coord =
                        Coord::new(coord.column, coord.row + 1);
                }
                current_row = remaining_rows.pop().unwrap_or_default();
            }

            // type characters on the bottom row while the head is not returning.
            if !returning && !head_retired {
                for _ in 0..print_speed {
                    let Some(idx) = current_row.pop() else { break };
                    let column = terminal.characters[idx].input_coord.column;
                    terminal.characters[idx].motion.current_coord = Coord::new(column, 1);
                    terminal.characters[idx].animation.activate_scene("typed");
                    let char_id = terminal.characters[idx].character_id;
                    terminal.set_character_visibility(char_id, true);
                    typed.push(idx);
                    terminal.characters[head_idx].motion.current_coord =
                        Coord::new(column + 1, 1);
                }
                if current_row.is_empty() {
                    if remaining_rows.is_empty() {
                        // last row printed: retire the head.
                        terminal.set_character_visibility(head_id, false);
                        head_retired = true;
                    } else {
                        // eased carriage return back to column 1.
                        let start = terminal.characters[head_idx].motion.current_coord;
                        let head = &mut terminal.characters[head_idx];
                        let path = head.motion.new_path(
                            "return",
                            self.print_head_return_speed,
                            Some(easing::in_out_quad),
                        );
                        path.add_waypoint(start);
                        path.add_waypoint(Coord::new(1, 1));
                        head.motion.activate_path("return");
                        returning = true;
                    }
                }
            }

            terminal.tick();
            frames.push(terminal.get_formatted_output_string());

            // done once everything is printed and every reveal has settled.
            if head_retired {
                let settled = typed.iter().all(|&idx| {
                    terminal.characters[idx].animation.active_scene_is_complete()
                });
                if settled {
                    break;
                }
            }
        }

        frames
    }
}
