use crate::utils::graphics::Style;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterVisual {
    pub symbol: char,
    pub style: Style,
}

impl CharacterVisual {
    pub fn new(symbol: char, style: Style) -> Self {
        Self { symbol, style }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub visual: CharacterVisual,
    pub duration: u32,
}

impl Frame {
    pub fn new(visual: CharacterVisual, duration: u32) -> Self {
        Self {
            visual,
            duration: duration.max(1),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Scene {
    frames: Vec<Frame>,
    frame_index: usize,
    frame_tick: u32,
    looping: bool,
    active: bool,
    finished: bool,
}

impl Scene {
    pub fn new(looping: bool) -> Self {
        Self {
            frames: Vec::new(),
            frame_index: 0,
            frame_tick: 0,
            looping,
            active: false,
            finished: false,
        }
    }

    pub fn with_frames(frames: Vec<Frame>, looping: bool) -> Self {
        Self {
            frames,
            frame_index: 0,
            frame_tick: 0,
            looping,
            active: false,
            finished: false,
        }
    }

    pub fn add_frame(&mut self, frame: Frame) {
        self.frames.push(frame);
    }

    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    pub fn activate(&mut self) -> bool {
        if self.frames.is_empty() {
            return false;
        }

        self.frame_index = 0;
        self.frame_tick = 0;
        self.active = true;
        self.finished = false;
        true
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn current_visual(&self) -> Option<&CharacterVisual> {
        self.frames
            .get(self.frame_index)
            .map(|frame| &frame.visual)
    }

    pub fn step(&mut self) -> Option<CharacterVisual> {
        if !self.active || self.frames.is_empty() {
            return None;
        }

        let frame = self.frames[self.frame_index].clone();
        self.frame_tick += 1;

        if self.frame_tick >= frame.duration {
            self.frame_tick = 0;
            self.frame_index += 1;

            if self.frame_index >= self.frames.len() {
                if self.looping {
                    self.frame_index = 0;
                } else {
                    self.frame_index = self.frames.len() - 1;
                    self.active = false;
                    self.finished = true;
                }
            }
        }

        Some(frame.visual)
    }
}

#[derive(Debug, Clone)]
pub struct Animation {
    current: CharacterVisual,
    active_scene: Option<Scene>,
}

impl Animation {
    pub fn new(symbol: char, style: Style) -> Self {
        Self {
            current: CharacterVisual::new(symbol, style),
            active_scene: None,
        }
    }

    pub fn current_visual(&self) -> &CharacterVisual {
        &self.current
    }

    pub fn set_appearance(&mut self, symbol: char, style: Style) {
        self.current = CharacterVisual::new(symbol, style);
    }

    pub fn activate_scene(&mut self, mut scene: Scene) -> bool {
        if !scene.activate() {
            return false;
        }

        if let Some(visual) = scene.current_visual() {
            self.current = visual.clone();
        }

        self.active_scene = Some(scene);
        true
    }

    pub fn active_scene(&self) -> Option<&Scene> {
        self.active_scene.as_ref()
    }

    pub fn active_scene_mut(&mut self) -> Option<&mut Scene> {
        self.active_scene.as_mut()
    }

    pub fn deactivate(&mut self) {
        if let Some(scene) = &mut self.active_scene {
            scene.deactivate();
        }
        self.active_scene = None;
    }

    pub fn step(&mut self) -> Option<CharacterVisual> {
        let scene = self.active_scene.as_mut()?;
        let visual = scene.step()?;
        self.current = visual.clone();
        Some(visual)
    }
}
