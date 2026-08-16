
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const BLACK: Self = Self::new(0, 0, 0);
    pub const WHITE: Self = Self::new(255, 255, 255);
    pub const RED: Self = Self::new(255, 0, 0);
    pub const GREEN: Self = Self::new(0, 255, 0);
    pub const BLUE: Self = Self::new(0, 0, 255);
    pub const CYAN: Self = Self::new(0, 255, 255);
    pub const MAGENTA: Self = Self::new(255, 0, 255);
    pub const YELLOW: Self = Self::new(255, 255, 0);

    pub fn fg_ansi(&self) -> String {
        format!("38;2;{};{};{}", self.r, self.g, self.b)
    }

    pub fn bg_ansi(&self) -> String {
        format!("48;2;{};{};{}", self.r, self.g, self.b)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorPair {
    pub fg: Color,
    pub bg: Color,
}

impl ColorPair {
    pub const fn new(fg: Color, bg: Color) -> Self {
        Self { fg, bg }
    }

    pub fn fg_ansi(&self) -> String {
        self.fg.fg_ansi()
    }

    pub fn bg_ansi(&self) -> String {
        self.bg.bg_ansi()
    }
}

impl Default for ColorPair {
    fn default() -> Self {
        Self::new(Color::WHITE, Color::BLACK)
    }
}

#[derive(Clone, Debug)]
pub struct Gradient {
    stops: Vec<(f32, Color)>,
}

impl Gradient {
    pub fn new() -> Self {
        Self { stops: Vec::new() }
    }

    pub fn add_stop(mut self, position: f32, color: Color) -> Self {
        self.stops.push((position, color));
        self
    }

    pub fn color_at(&self, t: f32) -> Color {
        if self.stops.is_empty() {
            return Color::WHITE;
        }

        let t = t.clamp(0.0, 1.0);
        let mut lower = self.stops[0];
        let mut upper = self.stops[self.stops.len() - 1];

        for pair in &self.stops {
            if pair.0 <= t {
                lower = *pair;
            }
            if pair.0 >= t {
                upper = *pair;
                break;
            }
        }

        if upper.0 == lower.0 {
            return lower.1;
        }

        let k = (t - lower.0) / (upper.0 - lower.0);
        Color::new(
            lerp_channel(lower.1.r, upper.1.r, k),
            lerp_channel(lower.1.g, upper.1.g, k),
            lerp_channel(lower.1.b, upper.1.b, k),
        )
    }
}

fn lerp_channel(a: u8, b: u8, t: f32) -> u8 {
    let value = a as f32 + (b as f32 - a as f32) * t;
    value.round().clamp(0.0, 255.0) as u8
}
