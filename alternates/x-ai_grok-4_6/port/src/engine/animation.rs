use std::collections::HashMap;
use std::fmt;

use crate::utils::easing::Ease;
use crate::utils::graphics::{Color, ColorPair, Gradient};
use crate::utils::round_half_even;

#[derive(Clone, Debug)]
pub struct CharacterVisual {
    pub symbol: String,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub reverse: bool,
    pub hidden: bool,
    pub strike: bool,
    pub colors: Option<ColorPair>,
    pub formatted_symbol: String,
}

impl CharacterVisual {
    pub fn new(symbol: impl Into<String>, colors: Option<ColorPair>) -> Self {
        let mut visual = Self {
            symbol: symbol.into(),
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            blink: false,
            reverse: false,
            hidden: false,
            strike: false,
            colors,
            formatted_symbol: String::new(),
        };
        visual.refresh();
        visual
    }

    pub fn refresh(&mut self) {
        self.formatted_symbol = self.format_symbol();
    }

    pub fn format_symbol(&self) -> String {
        let mut formatting = String::new();
        if self.bold {
            formatting.push_str("\x1b[1m");
        }
        if self.italic {
            formatting.push_str("\x1b[3m");
        }
        if self.underline {
            formatting.push_str("\x1b[4m");
        }
        if self.blink {
            formatting.push_str("\x1b[5m");
        }
        if self.reverse {
            formatting.push_str("\x1b[7m");
        }
        if self.hidden {
            formatting.push_str("\x1b[8m");
        }
        if self.strike {
            formatting.push_str("\x1b[9m");
        }
        if let Some(colors) = &self.colors {
            if let Some(fg) = colors.fg {
                formatting.push_str(&fg.fg_sgr());
            }
            if let Some(bg) = colors.bg {
                formatting.push_str(&bg.bg_sgr());
            }
        }
        if formatting.is_empty() {
            return self.symbol.clone();
        }
        formatting.push_str(&self.symbol);
        formatting.push_str("\x1b[0m");
        formatting
    }
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub character_visual: CharacterVisual,
    pub duration: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneSync {
    Color,
    Symbol,
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub scene_id: String,
    pub frames: Vec<Frame>,
    pub is_looping: bool,
    pub sync: Option<SceneSync>,
    pub ease: Option<Ease>,
    pub complete: bool,
    current_frame_index: usize,
    hold: u32,
    elapsed: u32,
}

impl Scene {
    pub fn new(scene_id: impl Into<String>) -> Self {
        Self {
            scene_id: scene_id.into(),
            frames: Vec::new(),
            is_looping: false,
            sync: None,
            ease: None,
            complete: false,
            current_frame_index: 0,
            hold: 0,
            elapsed: 0,
        }
    }

    pub fn add_frame(&mut self, symbol: &str, duration: u32, colors: Option<ColorPair>) {
        let duration = duration.max(1);
        self.frames.push(Frame {
            character_visual: CharacterVisual::new(symbol, colors),
            duration,
        });
    }

    pub fn apply_gradient_to_symbols(
        &mut self,
        gradient: &Gradient,
        symbols: &str,
        duration: u32,
    ) {
        let chars: Vec<char> = symbols.chars().collect();
        if chars.is_empty() || gradient.is_empty() {
            return;
        }
        for (index, color) in gradient.spectrum().iter().enumerate() {
            let ch = chars[index % chars.len()];
            self.add_frame(&ch.to_string(), duration, Some(ColorPair::fg(*color)));
        }
    }

    pub fn reset(&mut self) {
        self.current_frame_index = 0;
        self.hold = 0;
        self.elapsed = 0;
        self.complete = self.frames.is_empty();
    }

    pub fn current_visual(&self) -> Option<&CharacterVisual> {
        self.frames
            .get(self.current_frame_index)
            .map(|frame| &frame.character_visual)
    }

    pub fn advance(&mut self) {
        if self.frames.is_empty() {
            self.complete = true;
            return;
        }
        if self.complete && !self.is_looping {
            return;
        }
        if let Some(ease) = self.ease {
            let total: u32 = self.frames.iter().map(|frame| frame.duration).sum();
            if total == 0 {
                self.complete = true;
                return;
            }
            self.elapsed = self.elapsed.saturating_add(1);
            if self.elapsed >= total {
                if self.is_looping {
                    self.elapsed = 0;
                    self.complete = false;
                } else {
                    self.elapsed = total;
                    self.complete = true;
                    self.current_frame_index = self.frames.len() - 1;
                    return;
                }
            }
            let t = ease.apply(self.elapsed as f64 / total as f64);
            let last = (self.frames.len() - 1) as f64;
            let idx = round_half_even(t * last).clamp(0, last as i64) as usize;
            self.current_frame_index = idx;
            return;
        }

        let duration = self.frames[self.current_frame_index].duration;
        self.hold = self.hold.saturating_add(1);
        if self.hold >= duration {
            self.hold = 0;
            if self.current_frame_index + 1 >= self.frames.len() {
                if self.is_looping {
                    self.current_frame_index = 0;
                    self.complete = false;
                } else {
                    self.complete = true;
                }
            } else {
                self.current_frame_index += 1;
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Animation {
    pub scenes: HashMap<String, Scene>,
    pub scene_ids: Vec<String>,
    pub active_scene: Option<String>,
    pub current_character_visual: CharacterVisual,
    pub input_symbol: String,
}

impl Animation {
    pub fn new(input_symbol: impl Into<String>) -> Self {
        let input_symbol = input_symbol.into();
        Self {
            scenes: HashMap::new(),
            scene_ids: Vec::new(),
            active_scene: None,
            current_character_visual: CharacterVisual::new(input_symbol.clone(), None),
            input_symbol,
        }
    }

    pub fn new_scene(&mut self, scene_id: impl Into<String>) -> &mut Scene {
        let scene_id = scene_id.into();
        if !self.scenes.contains_key(&scene_id) {
            self.scene_ids.push(scene_id.clone());
            self.scenes
                .insert(scene_id.clone(), Scene::new(scene_id.clone()));
        }
        self.scenes.get_mut(&scene_id).expect("scene just inserted")
    }

    pub fn query_scene(&self, scene_id: &str) -> Result<&Scene, AnimationError> {
        self.scenes
            .get(scene_id)
            .ok_or_else(|| AnimationError::SceneNotFound(scene_id.to_string()))
    }

    pub fn activate_scene(&mut self, scene_id: &str) {
        if let Some(scene) = self.scenes.get_mut(scene_id) {
            scene.reset();
            if let Some(visual) = scene.current_visual().cloned() {
                self.current_character_visual = visual;
            }
            self.active_scene = Some(scene_id.to_string());
        }
    }

    pub fn deactivate_scene(&mut self) {
        self.active_scene = None;
    }

    pub fn set_appearance(&mut self, symbol: &str, colors: Option<ColorPair>) {
        self.current_character_visual = CharacterVisual::new(symbol, colors);
    }

    pub fn step_animation(&mut self) {
        let Some(scene_id) = self.active_scene.clone() else {
            return;
        };
        let Some(scene) = self.scenes.get_mut(&scene_id) else {
            return;
        };
        scene.advance();
        if let Some(visual) = scene.current_visual().cloned() {
            self.current_character_visual = visual;
        }
        if scene.complete && !scene.is_looping {
            self.active_scene = None;
        }
    }

    pub fn active_scene_is_complete(&self) -> bool {
        match &self.active_scene {
            Some(id) => self.scenes.get(id).map(|s| s.complete).unwrap_or(true),
            None => true,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active_scene.is_some()
    }
}

#[derive(Clone, Debug)]
pub enum AnimationError {
    DuplicateSceneId(String),
    SceneNotFound(String),
    ActivateEmptyScene(String),
    FrameDuration(u32),
}

impl fmt::Display for AnimationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateSceneId(id) => write!(f, "duplicate scene id '{id}'"),
            Self::SceneNotFound(id) => write!(f, "scene '{id}' not found"),
            Self::ActivateEmptyScene(id) => write!(f, "cannot activate empty scene '{id}'"),
            Self::FrameDuration(d) => write!(f, "invalid frame duration {d}"),
        }
    }
}

impl std::error::Error for AnimationError {}

impl ColorPair {
    pub fn into_option(self) -> Option<ColorPair> {
        if self.fg.is_none() && self.bg.is_none() {
            None
        } else {
            Some(self)
        }
    }
}
