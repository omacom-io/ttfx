//! CharacterVisual, Frame, Scene, and Animation, ported from engine/animation.py.
//! Scene/Animation stepping that fires events lives on EngineCtx (ctx.rs);
//! everything here is state plus event-free logic.

use crate::utils::ansi::{self, ColorCode};
use crate::utils::easing::Easing;
use crate::utils::graphics::{Color, ColorPair, Gradient};
use crate::utils::hexterm;
use crate::utils::ordered_map::OrderedMap;

/// Handling of preexisting SGR colors in the input (TerminalConfig option).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExistingColorHandling {
    Always,
    Dynamic,
    Ignore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMetric {
    Distance,
    Step,
}

/// animation.CharacterVisual with the formatted ANSI string precomputed.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterVisual {
    pub symbol: String,
    pub bold: bool,
    pub dim: bool, // stored but never emitted, faithfully
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub reverse: bool,
    pub hidden: bool,
    pub strike: bool,
    pub colors: Option<ColorPair>,
    pub fg_color_code: Option<ColorCode>,
    pub bg_color_code: Option<ColorCode>,
    pub formatted_symbol: String,
}

#[derive(Debug, Clone, Default)]
pub struct VisualParams {
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub reverse: bool,
    pub hidden: bool,
    pub strike: bool,
    pub colors: Option<ColorPair>,
    pub fg_color_code: Option<ColorCode>,
    pub bg_color_code: Option<ColorCode>,
}

impl CharacterVisual {
    pub fn new(symbol: &str, p: VisualParams) -> Self {
        let mut vis = CharacterVisual {
            symbol: symbol.to_string(),
            bold: p.bold,
            dim: p.dim,
            italic: p.italic,
            underline: p.underline,
            blink: p.blink,
            reverse: p.reverse,
            hidden: p.hidden,
            strike: p.strike,
            colors: p.colors,
            fg_color_code: p.fg_color_code,
            bg_color_code: p.bg_color_code,
            formatted_symbol: String::new(),
        };
        vis.formatted_symbol = vis.format_symbol();
        vis
    }

    pub fn plain(symbol: &str) -> Self {
        CharacterVisual::new(symbol, VisualParams::default())
    }

    /// SGR emission in upstream's fixed order; `dim` intentionally omitted;
    /// bare symbol when nothing applies.
    fn format_symbol(&self) -> String {
        let mut fmt = String::new();
        if self.bold {
            fmt.push_str(ansi::BOLD);
        }
        if self.italic {
            fmt.push_str(ansi::ITALIC);
        }
        if self.underline {
            fmt.push_str(ansi::UNDERLINE);
        }
        if self.blink {
            fmt.push_str(ansi::BLINK);
        }
        if self.reverse {
            fmt.push_str(ansi::REVERSE);
        }
        if self.hidden {
            fmt.push_str(ansi::HIDDEN);
        }
        if self.strike {
            fmt.push_str(ansi::STRIKETHROUGH);
        }
        if let Some(code) = &self.fg_color_code {
            ansi::fg(code, &mut fmt);
        }
        if let Some(code) = &self.bg_color_code {
            ansi::bg(code, &mut fmt);
        }
        if fmt.is_empty() {
            self.symbol.clone()
        } else {
            format!("{fmt}{}{}", self.symbol, ansi::RESET_ALL)
        }
    }
}

/// animation.Frame. Frames live in Scene.all_frames (stable storage);
/// Scene.frames / Scene.played_frames hold indices into it, preserving the
/// upstream object-identity semantics of frame_index_map.
#[derive(Debug, Clone)]
pub struct Frame {
    pub character_visual: CharacterVisual,
    pub duration: i64,
    pub ticks_elapsed: i64,
}

/// animation.Scene.
#[derive(Debug, Clone)]
pub struct Scene {
    pub scene_id: String,
    pub is_looping: bool,
    pub sync: Option<SyncMetric>,
    pub ease: Option<Easing>,
    pub no_color: bool,
    pub use_xterm_colors: bool,
    /// Stable frame storage; never reordered.
    pub all_frames: Vec<Frame>,
    /// Remaining frame queue (indices into all_frames).
    pub frames: Vec<usize>,
    /// Played frames (indices into all_frames).
    pub played_frames: Vec<usize>,
    /// Tick index -> frame index (upstream frame_index_map).
    pub frame_index_map: Vec<usize>,
    pub easing_total_steps: i64,
    pub easing_current_step: i64,
    pub preexisting_colors: Option<ColorPair>,
    pub preexisting_bold: bool,
}

impl Scene {
    pub fn new(
        scene_id: &str,
        is_looping: bool,
        sync: Option<SyncMetric>,
        ease: Option<Easing>,
        no_color: bool,
        use_xterm_colors: bool,
    ) -> Self {
        Scene {
            scene_id: scene_id.to_string(),
            is_looping,
            sync,
            ease,
            no_color,
            use_xterm_colors,
            all_frames: Vec::new(),
            frames: Vec::new(),
            played_frames: Vec::new(),
            frame_index_map: Vec::new(),
            easing_total_steps: 0,
            easing_current_step: 0,
            preexisting_colors: None,
            preexisting_bold: false,
        }
    }

    /// Scene._get_color_code. Upstream memoizes into a process-global ClassVar
    /// dict; the memo is value-transparent so we just recompute.
    fn get_color_code(&self, color: Option<&Color>) -> Option<ColorCode> {
        let color = color?;
        if self.no_color {
            return None;
        }
        if self.use_xterm_colors {
            if let Some(code) = color.xterm_color {
                return Some(ColorCode::Xterm(code));
            }
            return Some(ColorCode::Xterm(hexterm::hex_to_xterm(&color.rgb_color)));
        }
        Some(ColorCode::Rgb(color.rgb_color.clone()))
    }

    /// Scene.add_frame with the preexisting-color/bold overrides.
    pub fn add_frame(&mut self, symbol: &str, duration: i64, mut params: VisualParams) -> Result<(), String> {
        if let Some(pre) = &self.preexisting_colors {
            params.colors = Some(pre.clone());
        }
        if self.preexisting_bold {
            params.bold = true;
        }
        if let Some(colors) = &params.colors {
            params.fg_color_code = self.get_color_code(colors.fg_color.as_ref());
            params.bg_color_code = self.get_color_code(colors.bg_color.as_ref());
        } else {
            params.fg_color_code = None;
            params.bg_color_code = None;
        }
        if duration < 1 {
            return Err(format!("Frame duration must be at least 1. Received: {duration}"));
        }
        let visual = CharacterVisual::new(symbol, params);
        let frame_index = self.all_frames.len();
        self.all_frames.push(Frame { character_visual: visual, duration, ticks_elapsed: 0 });
        self.frames.push(frame_index);
        for _ in 0..duration {
            self.frame_index_map.push(frame_index);
            self.easing_total_steps += 1;
        }
        Ok(())
    }

    /// Scene.activate: first frame's visual, error when empty.
    pub fn activate(&self) -> Result<CharacterVisual, String> {
        match self.frames.first() {
            Some(&idx) => Ok(self.all_frames[idx].character_visual.clone()),
            None => Err(format!("Scene {} has no frames.", self.scene_id)),
        }
    }

    /// Scene.get_next_visual: tick the head frame, retiring it (and looping)
    /// exactly as upstream.
    pub fn get_next_visual(&mut self) -> CharacterVisual {
        let head = self.frames[0];
        let next_visual = self.all_frames[head].character_visual.clone();
        self.all_frames[head].ticks_elapsed += 1;
        if self.all_frames[head].ticks_elapsed == self.all_frames[head].duration {
            self.all_frames[head].ticks_elapsed = 0;
            self.played_frames.push(self.frames.remove(0));
            if self.is_looping && self.frames.is_empty() {
                self.frames.append(&mut self.played_frames);
            }
        }
        next_visual
    }

    /// Scene.apply_gradient_to_symbols with the exact cyclic_distribution
    /// generator semantics (repeat factor + overflow-remainder rule).
    pub fn apply_gradient_to_symbols(
        &mut self,
        symbols: &[String],
        duration: i64,
        fg_gradient: Option<&Gradient>,
        bg_gradient: Option<&Gradient>,
    ) -> Result<(), String> {
        fn cyclic_distribution<T: Clone, R: Clone>(larger: &[T], smaller: &[R]) -> Vec<(T, R)> {
            let repeat_factor = larger.len() / smaller.len();
            let mut overflow_count = larger.len() % smaller.len();
            let mut overflow_used = false;
            let mut smaller_index = 0usize;
            let mut current_repeat_factor = 0usize;
            let mut out = Vec::with_capacity(larger.len());
            for element in larger {
                if current_repeat_factor >= repeat_factor {
                    if overflow_count > 0 {
                        if overflow_used {
                            smaller_index += 1;
                            current_repeat_factor = 0;
                            overflow_used = false;
                        } else {
                            overflow_used = true;
                            overflow_count -= 1;
                        }
                    } else {
                        smaller_index += 1;
                        current_repeat_factor = 0;
                    }
                }
                current_repeat_factor += 1;
                out.push((element.clone(), smaller[smaller_index].clone()));
            }
            out
        }

        let fg_has = fg_gradient.is_some_and(|g| !g.spectrum.is_empty());
        let bg_has = bg_gradient.is_some_and(|g| !g.spectrum.is_empty());
        if fg_gradient.is_none() && bg_gradient.is_none() {
            return Err("Foreground and background gradient are None. At least one gradient must be provided.".into());
        }
        if !fg_has && !bg_has {
            return Err(
                "Foreground and background gradient are empty. At least one gradient must have at least one color."
                    .into(),
            );
        }
        for symbol in symbols {
            if symbol.chars().count() > 1 {
                return Err(format!("Symbol must be a string with a length of 1. Received: `{symbol}`."));
            }
        }
        let color_pairs: Vec<ColorPair> = if fg_has && bg_has {
            let fg = &fg_gradient.unwrap().spectrum;
            let bg = &bg_gradient.unwrap().spectrum;
            if fg.len() >= bg.len() {
                cyclic_distribution(fg, bg)
                    .into_iter()
                    .map(|(f, b)| ColorPair::new(Some(f), Some(b)))
                    .collect()
            } else {
                cyclic_distribution(bg, fg)
                    .into_iter()
                    .map(|(b, f)| ColorPair::new(Some(f), Some(b)))
                    .collect()
            }
        } else if fg_has {
            fg_gradient.unwrap().spectrum.iter().map(|c| ColorPair::new(Some(c.clone()), None)).collect()
        } else {
            bg_gradient.unwrap().spectrum.iter().map(|c| ColorPair::new(None, Some(c.clone()))).collect()
        };

        if symbols.len() >= color_pairs.len() {
            for (symbol, colors) in cyclic_distribution(symbols, &color_pairs) {
                self.add_frame(&symbol, duration, VisualParams { colors: Some(colors), ..Default::default() })?;
            }
        } else {
            for (colors, symbol) in cyclic_distribution(&color_pairs, symbols) {
                self.add_frame(&symbol, duration, VisualParams { colors: Some(colors), ..Default::default() })?;
            }
        }
        Ok(())
    }

    /// Scene.reset_scene: restore played + remaining frames in original order
    /// (played first), zero tick counters and the easing step.
    pub fn reset_scene(&mut self) {
        // Remaining frames get ticks_elapsed zeroed as they move to played;
        // already-played frames were zeroed when they retired.
        let remaining: Vec<usize> = self.frames.drain(..).collect();
        for &idx in &remaining {
            self.all_frames[idx].ticks_elapsed = 0;
            self.played_frames.push(idx);
        }
        self.frames.extend(self.played_frames.drain(..));
        self.easing_current_step = 0;
    }
}

/// engine/animation.py Animation: per-character animation state.
#[derive(Debug, Clone)]
pub struct Animation {
    pub scenes: OrderedMap<Scene>,
    pub active_scene: Option<String>,
    pub use_xterm_colors: bool,
    pub no_color: bool,
    pub existing_color_handling: ExistingColorHandling,
    pub input_fg_color: Option<Color>,
    pub input_bg_color: Option<Color>,
    pub input_bold: bool,
    pub active_scene_current_step: i64,
    pub current_character_visual: CharacterVisual,
}

impl Animation {
    pub fn new(input_symbol: &str) -> Self {
        Animation {
            scenes: OrderedMap::new(),
            active_scene: None,
            use_xterm_colors: false,
            no_color: false,
            existing_color_handling: ExistingColorHandling::Ignore,
            input_fg_color: None,
            input_bg_color: None,
            input_bold: false,
            active_scene_current_step: 0,
            current_character_visual: CharacterVisual::plain(input_symbol),
        }
    }

    /// Animation._get_color_code (identical logic to Scene's; the upstream
    /// per-instance memo is value-transparent and omitted).
    pub fn get_color_code(&mut self, color: Option<&Color>) -> Option<ColorCode> {
        let color = color?;
        if self.no_color {
            return None;
        }
        if self.use_xterm_colors {
            if let Some(code) = color.xterm_color {
                return Some(ColorCode::Xterm(code));
            }
            return Some(ColorCode::Xterm(hexterm::hex_to_xterm(&color.rgb_color)));
        }
        Some(ColorCode::Rgb(color.rgb_color.clone()))
    }

    /// Animation.new_scene: auto-ids are stringified integers probing upward;
    /// duplicate explicit ids silently overwrite (faithful).
    pub fn new_scene(
        &mut self,
        is_looping: bool,
        sync: Option<SyncMetric>,
        ease: Option<Easing>,
        scene_id: &str,
        uses_input_preexisting_colors: bool,
    ) -> String {
        let scene_id = if scene_id.is_empty() {
            let mut current_id = self.scenes.len();
            loop {
                let candidate = current_id.to_string();
                if !self.scenes.contains_key(&candidate) {
                    break candidate;
                }
                current_id += 1;
            }
        } else {
            scene_id.to_string()
        };
        let (preexisting_colors, preexisting_bold) =
            if self.existing_color_handling == ExistingColorHandling::Always && uses_input_preexisting_colors {
                (
                    Some(ColorPair::new(self.input_fg_color.clone(), self.input_bg_color.clone())),
                    self.input_bold,
                )
            } else {
                (None, false)
            };
        let mut scene = Scene::new(&scene_id, is_looping, sync, ease, self.no_color, self.use_xterm_colors);
        scene.preexisting_colors = preexisting_colors;
        scene.preexisting_bold = preexisting_bold;
        self.scenes.insert(scene_id.clone(), scene);
        scene_id
    }

    /// Animation.active_scene_is_complete: no scene, no remaining frames, or looping.
    pub fn active_scene_is_complete(&self) -> bool {
        match &self.active_scene {
            None => true,
            Some(id) => {
                let scene = self.scenes.get(id).expect("active scene must exist");
                scene.frames.is_empty() || scene.is_looping
            }
        }
    }

    /// Animation.set_appearance.
    pub fn set_appearance(
        &mut self,
        input_symbol: &str,
        uses_input_preexisting_colors: bool,
        symbol: Option<&str>,
        colors: Option<ColorPair>,
    ) {
        let symbol = symbol.unwrap_or(input_symbol);
        let mut colors = colors.unwrap_or_default();
        let mut bold = false;
        if self.existing_color_handling == ExistingColorHandling::Always && uses_input_preexisting_colors {
            colors = ColorPair::new(self.input_fg_color.clone(), self.input_bg_color.clone());
            bold = self.input_bold;
        }
        let fg_code = self.get_color_code(colors.fg_color.as_ref());
        let bg_code = self.get_color_code(colors.bg_color.as_ref());
        self.current_character_visual = CharacterVisual::new(
            symbol,
            VisualParams {
                bold,
                colors: Some(colors),
                fg_color_code: fg_code,
                bg_color_code: bg_code,
                ..Default::default()
            },
        );
    }

    /// Animation.adjust_color_brightness: hand-rolled RGB->HSL->RGB with
    /// round() (banker's) at the end — unlike shift_color_towards's truncation.
    pub fn adjust_color_brightness(color: &Color, brightness: f64) -> Color {
        use crate::utils::pycompat::round_half_even;

        fn hue_to_rgb(lightness_scaled: f64, color_intensity: f64, mut hue_value: f64) -> f64 {
            if hue_value < 0.0 {
                hue_value += 1.0;
            }
            if hue_value > 1.0 {
                hue_value -= 1.0;
            }
            if hue_value < 1.0 / 6.0 {
                return lightness_scaled + (color_intensity - lightness_scaled) * 6.0 * hue_value;
            }
            if hue_value < 1.0 / 2.0 {
                return color_intensity;
            }
            if hue_value < 2.0 / 3.0 {
                return lightness_scaled + (color_intensity - lightness_scaled) * (2.0 / 3.0 - hue_value) * 6.0;
            }
            lightness_scaled
        }

        let (r, g, b) = color.rgb_ints();
        let normalized_red = r as f64 / 255.0;
        let normalized_green = g as f64 / 255.0;
        let normalized_blue = b as f64 / 255.0;

        let max_val = normalized_red.max(normalized_green).max(normalized_blue);
        let min_val = normalized_red.min(normalized_green).min(normalized_blue);
        let mut lightness = (max_val + min_val) / 2.0;

        let lightness_threshold = 0.5;
        let (hue_value, saturation) = if max_val == min_val {
            (0.0, 0.0)
        } else {
            let diff = max_val - min_val;
            let saturation = if lightness > lightness_threshold {
                diff / (2.0 - max_val - min_val)
            } else {
                diff / (max_val + min_val)
            };
            let mut hue_value = if max_val == normalized_red {
                (normalized_green - normalized_blue) / diff + if normalized_green < normalized_blue { 6.0 } else { 0.0 }
            } else if max_val == normalized_green {
                (normalized_blue - normalized_red) / diff + 2.0
            } else {
                (normalized_red - normalized_green) / diff + 4.0
            };
            hue_value /= 6.0;
            (hue_value, saturation)
        };

        lightness = (lightness * brightness).min(1.0).max(0.0);

        let (red, green, blue) = if saturation == 0.0 {
            (lightness, lightness, lightness)
        } else {
            let color_intensity = if lightness < lightness_threshold {
                lightness * (1.0 + saturation)
            } else {
                lightness + saturation - lightness * saturation
            };
            let lightness_scaled = 2.0 * lightness - color_intensity;
            (
                hue_to_rgb(lightness_scaled, color_intensity, hue_value + 1.0 / 3.0),
                hue_to_rgb(lightness_scaled, color_intensity, hue_value),
                hue_to_rgb(lightness_scaled, color_intensity, hue_value - 1.0 / 3.0),
            )
        };

        let adjusted = format!(
            "{:02x}{:02x}{:02x}",
            round_half_even(red * 255.0),
            round_half_even(green * 255.0),
            round_half_even(blue * 255.0)
        );
        Color::from_hex(&adjusted).unwrap()
    }
}
