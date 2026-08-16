//! Scenes, frames and per-character animation state.

use std::collections::HashMap;

use crate::utils::graphics::ColorPair;

/// The symbol, colors and modes a character displays for one frame.
#[derive(Clone, Debug, PartialEq)]
pub struct CharacterVisual {
    pub symbol: char,
    pub bold: bool,
    pub colors: ColorPair,
}

impl Default for CharacterVisual {
    fn default() -> Self {
        CharacterVisual {
            symbol: ' ',
            bold: false,
            colors: ColorPair::default(),
        }
    }
}

impl CharacterVisual {
    pub fn new(symbol: char, bold: bool, colors: ColorPair) -> Self {
        CharacterVisual {
            symbol,
            bold,
            colors,
        }
    }

    /// Apply ANSI sequences for the active modes/colors around the symbol.
    pub fn formatted(&self) -> String {
        let mut out = String::new();
        let mut styled = false;
        if self.bold {
            out.push_str("\x1b[1m");
            styled = true;
        }
        if let Some(fg) = self.colors.fg {
            out.push_str(&fg.to_ansi_fg());
            styled = true;
        }
        if let Some(bg) = self.colors.bg {
            out.push_str(&bg.to_ansi_bg());
            styled = true;
        }
        out.push(self.symbol);
        if styled {
            out.push_str("\x1b[0m");
        }
        out
    }
}

/// One frame of a scene: a visual held for `duration` ticks.
#[derive(Clone, Debug)]
pub struct Frame {
    pub visual: CharacterVisual,
    pub duration: u32,
    pub ticks_elapsed: u32,
}

impl Frame {
    pub fn new(visual: CharacterVisual, duration: u32) -> Self {
        Frame {
            visual,
            duration: duration.max(1),
            ticks_elapsed: 0,
        }
    }
}

/// A sequence of frames stepped once per engine tick.
#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub scene_id: String,
    pub frames: Vec<Frame>,
    pub frame_index: usize,
    pub is_looping: bool,
    pub complete: bool,
}

impl Scene {
    pub fn new(scene_id: &str, is_looping: bool) -> Self {
        Scene {
            scene_id: scene_id.to_string(),
            frames: Vec::new(),
            frame_index: 0,
            is_looping,
            complete: false,
        }
    }

    pub fn add_frame(&mut self, symbol: char, duration: u32, colors: ColorPair, bold: bool) {
        self.frames
            .push(Frame::new(CharacterVisual::new(symbol, bold, colors), duration));
    }

    pub fn reset(&mut self) {
        self.frame_index = 0;
        self.complete = false;
        for frame in &mut self.frames {
            frame.ticks_elapsed = 0;
        }
    }

    /// Advance one tick and return the visual to display.
    pub fn get_next_visual(&mut self) -> Option<CharacterVisual> {
        if self.frames.is_empty() {
            self.complete = true;
            return None;
        }
        let last = self.frames.len() - 1;
        if self.complete {
            return Some(self.frames[last].visual.clone());
        }
        let idx = self.frame_index.min(last);
        let visual = self.frames[idx].visual.clone();
        let frame = &mut self.frames[idx];
        frame.ticks_elapsed += 1;
        if frame.ticks_elapsed >= frame.duration {
            frame.ticks_elapsed = 0;
            if idx < last {
                self.frame_index = idx + 1;
            } else if self.is_looping {
                self.frame_index = 0;
            } else {
                self.complete = true;
            }
        }
        Some(visual)
    }
}

/// Per-character animation: a set of scenes and the currently active one.
#[derive(Clone, Debug, Default)]
pub struct Animation {
    scenes: HashMap<String, Scene>,
    pub active_scene: Option<String>,
    pub current_visual: CharacterVisual,
}

impl Animation {
    pub fn new(symbol: char) -> Self {
        Animation {
            scenes: HashMap::new(),
            active_scene: None,
            current_visual: CharacterVisual::new(symbol, false, ColorPair::default()),
        }
    }

    pub fn new_scene(&mut self, scene_id: &str, is_looping: bool) -> &mut Scene {
        self.scenes
            .insert(scene_id.to_string(), Scene::new(scene_id, is_looping));
        self.scenes.get_mut(scene_id).expect("scene just inserted")
    }

    pub fn query_scene(&self, scene_id: &str) -> Option<&Scene> {
        self.scenes.get(scene_id)
    }

    pub fn query_scene_mut(&mut self, scene_id: &str) -> Option<&mut Scene> {
        self.scenes.get_mut(scene_id)
    }

    pub fn activate_scene(&mut self, scene_id: &str) {
        if let Some(scene) = self.scenes.get_mut(scene_id) {
            scene.reset();
            self.active_scene = Some(scene_id.to_string());
        }
    }

    pub fn deactivate_scene(&mut self) {
        self.active_scene = None;
    }

    /// Step the active scene one tick, updating the current visual.
    pub fn step_animation(&mut self) {
        let Some(scene_id) = self.active_scene.clone() else {
            return;
        };
        if let Some(scene) = self.scenes.get_mut(&scene_id) {
            if let Some(visual) = scene.get_next_visual() {
                self.current_visual = visual;
            }
        }
    }

    pub fn active_scene_is_complete(&self) -> bool {
        match &self.active_scene {
            Some(scene_id) => self
                .scenes
                .get(scene_id)
                .map(|s| s.complete && !s.is_looping)
                .unwrap_or(true),
            None => true,
        }
    }
}
