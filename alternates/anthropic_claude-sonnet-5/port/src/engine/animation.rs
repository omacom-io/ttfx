//! Scene, Frame, CharacterVisual, Animation (mirrors
//! terminaltexteffects/engine/animation.py, minus the effect-facing sync/
//! eased stepping helpers, which will land with the effect ports).

use std::collections::HashMap;

use crate::utils::graphics::ColorPair;

/// A single visual state for a character: its symbol plus text modes and
/// color, and the precomputed ANSI-formatted symbol ready for output.
#[derive(Debug, Clone)]
pub struct CharacterVisual {
    pub symbol: char,
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
    pub fn new(symbol: char) -> Self {
        let mut visual = CharacterVisual {
            symbol,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            blink: false,
            reverse: false,
            hidden: false,
            strike: false,
            colors: None,
            formatted_symbol: String::new(),
        };
        visual.formatted_symbol = visual.format_symbol();
        visual
    }

    /// Apply ANSI SGR sequences for any active modes/colors, mirroring
    /// `CharacterVisual.format_symbol`. `dim` is stored but, as upstream
    /// notes, is not currently emitted.
    pub fn format_symbol(&self) -> String {
        let mut codes: Vec<String> = Vec::new();
        if self.bold {
            codes.push("1".to_string());
        }
        if self.italic {
            codes.push("3".to_string());
        }
        if self.underline {
            codes.push("4".to_string());
        }
        if self.blink {
            codes.push("5".to_string());
        }
        if self.reverse {
            codes.push("7".to_string());
        }
        if self.hidden {
            codes.push("8".to_string());
        }
        if self.strike {
            codes.push("9".to_string());
        }
        if let Some(colors) = &self.colors {
            if let Some(fg) = colors.fg {
                codes.push(fg.fg_sgr());
            }
            if let Some(bg) = colors.bg {
                codes.push(bg.bg_sgr());
            }
        }
        if codes.is_empty() {
            self.symbol.to_string()
        } else {
            format!("\x1b[{}m{}\x1b[0m", codes.join(";"), self.symbol)
        }
    }
}

/// A frame is a visual held for `duration` animation ticks.
#[derive(Debug, Clone)]
pub struct Frame {
    pub visual: CharacterVisual,
    pub duration: u32,
}

/// A sequence of frames playable in order, optionally looping.
#[derive(Debug, Clone)]
pub struct Scene {
    pub id: String,
    pub frames: Vec<Frame>,
    pub is_looping: bool,
}

impl Scene {
    pub fn new(id: impl Into<String>) -> Self {
        Scene { id: id.into(), frames: Vec::new(), is_looping: false }
    }

    pub fn add_frame(&mut self, visual: CharacterVisual, duration: u32) {
        self.frames.push(Frame { visual, duration: duration.max(1) });
    }
}

/// Per-character animation state: registered scenes, the active scene, and
/// current playback position within it.
#[derive(Debug, Clone)]
pub struct Animation {
    pub scenes: HashMap<String, Scene>,
    pub active_scene_id: Option<String>,
    frame_index: usize,
    ticks_on_frame: u32,
    current_visual: CharacterVisual,
}

impl Animation {
    /// Construct with a single non-looping "default" scene holding one frame
    /// of the character's plain input symbol, matching an un-animated
    /// character's baseline appearance.
    pub fn new(input_symbol: char) -> Self {
        let default_visual = CharacterVisual::new(input_symbol);
        let mut default_scene = Scene::new("default");
        default_scene.add_frame(default_visual.clone(), 1);
        let mut scenes = HashMap::new();
        scenes.insert("default".to_string(), default_scene);
        Animation {
            scenes,
            active_scene_id: Some("default".to_string()),
            frame_index: 0,
            ticks_on_frame: 0,
            current_visual: default_visual,
        }
    }

    pub fn add_scene(&mut self, scene: Scene) {
        self.scenes.insert(scene.id.clone(), scene);
    }

    /// Activate a scene by id, resetting its playback position, mirroring
    /// `Animation.activate_scene`.
    pub fn activate_scene(&mut self, scene_id: &str) {
        if let Some(scene) = self.scenes.get(scene_id) {
            self.active_scene_id = Some(scene_id.to_string());
            self.frame_index = 0;
            self.ticks_on_frame = 0;
            if let Some(frame) = scene.frames.first() {
                self.current_visual = frame.visual.clone();
            }
        }
    }

    /// Advance the active scene by one animation tick, mirroring
    /// `Animation.step_animation`.
    pub fn step_animation(&mut self) {
        let Some(scene_id) = self.active_scene_id.clone() else { return };
        let Some(scene) = self.scenes.get(&scene_id) else { return };
        if scene.frames.is_empty() {
            return;
        }

        self.ticks_on_frame += 1;
        let current_duration = scene.frames[self.frame_index].duration;
        if self.ticks_on_frame >= current_duration {
            self.ticks_on_frame = 0;
            if self.frame_index + 1 < scene.frames.len() {
                self.frame_index += 1;
            } else if scene.is_looping {
                self.frame_index = 0;
            }
            // Non-looping scenes hold on the final frame once exhausted.
        }
        self.current_visual = scene.frames[self.frame_index].visual.clone();
    }

    pub fn current_visual(&self) -> &CharacterVisual {
        &self.current_visual
    }

    pub fn set_appearance(&mut self, symbol: char, colors: Option<ColorPair>) {
        let mut visual = CharacterVisual::new(symbol);
        visual.colors = colors;
        visual.formatted_symbol = visual.format_symbol();
        self.current_visual = visual;
    }
}
