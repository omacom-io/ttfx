
use crate::engine::canvas::CellStyle;

#[derive(Clone, Debug)]
pub struct CharacterVisual {
    pub symbol: String,
    pub style: CellStyle,
}

impl CharacterVisual {
    pub fn new(symbol: impl Into<String>, style: CellStyle) -> Self {
        Self {
            symbol: symbol.into(),
            style,
        }
    }

    pub fn render(&self) -> String {
        self.style.render_symbol(&self.symbol)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub duration: u32,
    pub visuals: Vec<CharacterVisual>,
}

impl Frame {
    pub fn new(duration: u32) -> Self {
        Self {
            duration,
            visuals: Vec::new(),
        }
    }

    pub fn add_visual(&mut self, visual: CharacterVisual) {
        self.visuals.push(visual);
    }

    pub fn render(&self) -> String {
        self.visuals.iter().map(CharacterVisual::render).collect()
    }
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub frames: Vec<Frame>,
}

impl Scene {
    pub fn new() -> Self {
        Self { frames: Vec::new() }
    }

    pub fn add_frame(&mut self, frame: Frame) {
        self.frames.push(frame);
    }
}

#[derive(Clone, Debug)]
pub struct Animation {
    pub scenes: Vec<Scene>,
}

impl Animation {
    pub fn new() -> Self {
        Self { scenes: Vec::new() }
    }

    pub fn add_scene(&mut self, scene: Scene) {
        self.scenes.push(scene);
    }
}
