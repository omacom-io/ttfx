use super::Effect;

use crate::engine::animation::{CharacterVisual, Scene};
use crate::engine::terminal::Terminal;
use crate::utils::easing;
use crate::utils::geometry::{self, Coord};
use crate::utils::graphics::{Color, ColorPair};

/// Small self-contained xorshift64* PRNG. The swarm effect needs
/// reproducible pseudo-randomness for scatter/rally placement but no
/// `rng.rs` helper module exists yet in this skeleton, so it is inlined
/// here rather than invented as a shared utility.
struct SwarmRng {
    state: u64,
}

impl SwarmRng {
    fn new(seed: u64) -> Self {
        SwarmRng { state: seed | 1 }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Returns a value in `[lo, hi)`. Falls back to `lo` if the range is
    /// empty or inverted.
    fn range(&mut self, lo: i32, hi_exclusive: i32) -> i32 {
        if hi_exclusive <= lo {
            return lo;
        }
        let span = (hi_exclusive - lo) as u64;
        lo + (self.next_u64() % span) as i32
    }

    fn frac(&mut self) -> f64 {
        (self.next_u64() % 1_000_000) as f64 / 1_000_000.0
    }
}

/// Per-group rally point shared by a cluster of characters, plus the
/// off-canvas coordinate the cluster emerges from.
struct GroupInfo {
    start: Coord,
    rally: Coord,
}

/// Precomputed per-character trajectory: a two-leg path (start -> rally ->
/// final resting spot) plus how many frames it takes to traverse it.
struct CharPlan {
    start: Coord,
    rally: Coord,
    end: Coord,
    total_len: f64,
    frames_needed: usize,
    reached: bool,
}

/// Swarm effect: characters emerge off-canvas in small clusters, buzz
/// around a shared rally point, then settle into their final positions.
///
/// This is a from-scratch re-implementation against the simplified engine
/// primitives available in this port (no event system, no bezier-aware
/// `Path`/`Motion` stepping wired up yet), driving character position and
/// animation state directly frame-by-frame rather than relying on
/// `Motion::activate_path` + `Motion::step`.
pub struct Swarm;

impl Swarm {
    pub fn new() -> Self {
        Swarm
    }
}

impl Effect for Swarm {
    fn name(&self) -> &str {
        "swarm"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let mut terminal = Terminal::new(input);
        let mut rng = SwarmRng::new(0x9E37_79B9_7F4A_7C15);

        let width = terminal.canvas.width as i32;
        let height = terminal.canvas.height as i32;

        let ids: Vec<u32> = terminal.get_characters().iter().map(|c| c.id).collect();
        let char_count = ids.len();

        const GROUP_SIZE: usize = 6;
        let num_groups = ((char_count + GROUP_SIZE - 1) / GROUP_SIZE).max(1);

        let mut groups: Vec<GroupInfo> = Vec::with_capacity(num_groups);
        for _ in 0..num_groups {
            let edge = rng.range(0, 4);
            let start = match edge {
                0 => Coord::new(rng.range(0, width.max(1)), -(2 + rng.range(0, 3))),
                1 => Coord::new(rng.range(0, width.max(1)), height + 2 + rng.range(0, 3)),
                2 => Coord::new(-(2 + rng.range(0, 3)), rng.range(0, height.max(1))),
                _ => Coord::new(width + 2 + rng.range(0, 3), rng.range(0, height.max(1))),
            };
            let rally = Coord::new(rng.range(0, width.max(1)), rng.range(0, height.max(1)));
            groups.push(GroupInfo { start, rally });
        }

        let swarm_symbols = ['*', '+', 'x', 'o'];
        let swarm_colors = [
            Color::Ansi256(118),
            Color::Ansi256(154),
            Color::Ansi256(190),
            Color::Ansi256(226),
            Color::Ansi256(82),
        ];

        let mut plans: Vec<CharPlan> = Vec::with_capacity(char_count);

        for (idx, id) in ids.iter().enumerate() {
            let group = &groups[idx / GROUP_SIZE];

            let jitter_col = rng.range(-2, 3);
            let jitter_row = rng.range(-1, 2);
            let rally = Coord::new(
                (group.rally.column + jitter_col).clamp(0, (width.max(1) - 1).max(0)),
                (group.rally.row + jitter_row).clamp(0, (height.max(1) - 1).max(0)),
            );

            let end = terminal.get_character(*id).unwrap().input_coord;
            let speed = 0.5 + rng.frac() * 0.7;
            let seg1 = geometry::find_length_of_line(group.start, rally);
            let seg2 = geometry::find_length_of_line(rally, end);
            let total_len = seg1 + seg2;
            let frames_needed = ((total_len / speed).ceil() as usize).max(1);

            {
                let character = terminal.get_character_mut(*id).unwrap();
                character.motion.current_coord = group.start;
                character.motion.current_pos = (group.start.column as f64, group.start.row as f64);

                let symbol = swarm_symbols[idx % swarm_symbols.len()];
                let color = swarm_colors[idx % swarm_colors.len()];

                let mut visual_a = CharacterVisual::new(symbol);
                visual_a.colors = Some(ColorPair::new(Some(color), None));
                visual_a.formatted_symbol = visual_a.format_symbol();

                let symbol_b = swarm_symbols[(idx + 1) % swarm_symbols.len()];
                let mut visual_b = CharacterVisual::new(symbol_b);
                visual_b.colors = Some(ColorPair::new(Some(color), None));
                visual_b.formatted_symbol = visual_b.format_symbol();

                let mut swarm_scene = Scene::new("swarm");
                swarm_scene.is_looping = true;
                swarm_scene.add_frame(visual_a, 2);
                swarm_scene.add_frame(visual_b, 2);

                character.animation.add_scene(swarm_scene);
                character.animation.activate_scene("swarm");
            }

            plans.push(CharPlan {
                start: group.start,
                rally,
                end,
                total_len,
                frames_needed,
                reached: false,
            });
        }

        let max_frames = plans.iter().map(|p| p.frames_needed).max().unwrap_or(1);
        let mut frames = Vec::with_capacity(max_frames + 2);

        for frame_idx in 0..=max_frames {
            for (i, id) in ids.iter().enumerate() {
                let plan = &mut plans[i];
                let t = (frame_idx as f64 / plan.frames_needed as f64).min(1.0);

                let (col_f, row_f) = if plan.total_len <= 0.0 || t >= 1.0 {
                    (plan.end.column as f64, plan.end.row as f64)
                } else if t < 0.5 {
                    let seg_t = easing::ease_in_out_quad(t / 0.5);
                    geometry::lerp(plan.start, plan.rally, seg_t)
                } else {
                    let seg_t = easing::ease_in_out_quad((t - 0.5) / 0.5);
                    geometry::lerp(plan.rally, plan.end, seg_t)
                };

                let character = terminal.get_character_mut(*id).unwrap();
                character.motion.current_pos = (col_f, row_f);
                character.motion.current_coord = Coord::new(col_f.round() as i32, row_f.round() as i32);

                if t >= 1.0 && !plan.reached {
                    plan.reached = true;
                    character.animation.activate_scene("default");
                }
            }

            terminal.step_animation();
            frames.push(terminal.render());
        }

        frames
    }
}
