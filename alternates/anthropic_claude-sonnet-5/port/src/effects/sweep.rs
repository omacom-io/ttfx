
use super::Effect;
use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::terminal::Terminal;
use crate::utils::graphics::{Color, ColorPair, Gradient};

/// Sweeps a bright flash diagonally across the input, top-left to
/// bottom-right, revealing each character as the sweep line passes over it
/// and settling it into a horizontally-varying hue.
pub struct Sweep;

impl Sweep {
    pub fn new() -> Self {
        Sweep
    }
}

impl Effect for Sweep {
    fn name(&self) -> &str {
        "sweep"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);

        let width = terminal.canvas.width.max(1);

        const FLASH_STEPS: usize = 6;
        let flash_color = Color::Rgb(255, 255, 255);

        // Horizontal spread of settle colors used once the sweep has passed.
        let settle_gradient = Gradient::new(
            &[
                Color::Rgb(0, 255, 255),
                Color::Rgb(255, 0, 255),
                Color::Rgb(255, 255, 0),
            ],
            width,
        );

        let settle_color_for_column = |column: i32| -> Color {
            if settle_gradient.is_empty() {
                return flash_color;
            }
            let clamped = column.max(0) as usize;
            let ratio = clamped as f64 / width as f64;
            let idx = ((settle_gradient.len().saturating_sub(1)) as f64 * ratio).round() as usize;
            settle_gradient
                .get(idx.min(settle_gradient.len() - 1))
                .unwrap_or(flash_color)
        };

        let mut max_diagonal: i32 = 0;

        for character in terminal.get_characters_mut() {
            let diag = character.input_coord.column + character.input_coord.row;
            if diag > max_diagonal {
                max_diagonal = diag;
            }
            if character.input_symbol != ' ' {
                character.set_visibility(false);

                let final_color = settle_color_for_column(character.input_coord.column);
                let scene = build_sweep_scene(character.input_symbol, flash_color, final_color);
                character.animation.add_scene(scene);
            }
        }

        const TICKS_PER_DIAGONAL: i32 = 1;
        let total_ticks = max_diagonal * TICKS_PER_DIAGONAL + FLASH_STEPS as i32 + 2;

        let mut frames = Vec::new();
        for tick in 0..total_ticks {
            frames.push(terminal.render());

            for character in terminal.get_characters_mut() {
                if character.input_symbol == ' ' {
                    continue;
                }
                let diag = character.input_coord.column + character.input_coord.row;
                if tick == diag * TICKS_PER_DIAGONAL {
                    character.set_visibility(true);
                    character.animation.activate_scene("sweep");
                }
            }

            terminal.step_animation();
        }

        frames.push(terminal.render());
        frames
    }
}

/// Build the per-character "sweep" scene: a short flash from `flash_color`
/// into `final_color`, held thereafter since the scene does not loop.
fn build_sweep_scene(symbol: char, flash_color: Color, final_color: Color) -> Scene {
    let mut scene = Scene::new("sweep");
    let steps = 6usize;
    let gradient = Gradient::new(&[flash_color, final_color], steps);
    for color in &gradient.spectrum {
        let visual = make_visual(symbol, Some(ColorPair::new(Some(*color), None)));
        scene.add_frame(visual, 1);
    }
    scene
}

fn make_visual(symbol: char, colors: Option<ColorPair>) -> CharacterVisual {
    let mut visual = CharacterVisual::new(symbol);
    visual.colors = colors;
    visual.formatted_symbol = visual.format_symbol();
    visual
}
