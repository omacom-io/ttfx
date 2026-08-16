//! Color, ColorPair, Gradient (mirrors terminaltexteffects/utils/graphics.py,
//! ansitools/colorterm subset folded in here for the skeleton).

/// A terminal color, either an xterm-256 index or a truecolor RGB triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Ansi256(u8),
    Rgb(u8, u8, u8),
}

impl Color {
    /// SGR foreground color fragment, without leading ESC[ or trailing m.
    pub fn fg_sgr(&self) -> String {
        match self {
            Color::Ansi256(n) => format!("38;5;{n}"),
            Color::Rgb(r, g, b) => format!("38;2;{r};{g};{b}"),
        }
    }

    /// SGR background color fragment, without leading ESC[ or trailing m.
    pub fn bg_sgr(&self) -> String {
        match self {
            Color::Ansi256(n) => format!("48;5;{n}"),
            Color::Rgb(r, g, b) => format!("48;2;{r};{g};{b}"),
        }
    }

    fn as_rgb(&self) -> (f64, f64, f64) {
        match self {
            Color::Rgb(r, g, b) => (*r as f64, *g as f64, *b as f64),
            // Ansi256 indices are not interpolated in this skeleton; treat as
            // a neutral gray fallback so gradients over mixed color kinds
            // still produce something reasonable.
            Color::Ansi256(_) => (128.0, 128.0, 128.0),
        }
    }

    /// Linear interpolation between two colors (used by Gradient).
    pub fn lerp(a: Color, b: Color, t: f64) -> Color {
        let (ar, ag, ab) = a.as_rgb();
        let (br, bg, bb) = b.as_rgb();
        let r = (ar + (br - ar) * t).round().clamp(0.0, 255.0) as u8;
        let g = (ag + (bg - ag) * t).round().clamp(0.0, 255.0) as u8;
        let bl = (ab + (bb - ab) * t).round().clamp(0.0, 255.0) as u8;
        Color::Rgb(r, g, bl)
    }
}

/// A foreground/background color pairing, either half of which may be unset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ColorPair {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
}

impl ColorPair {
    pub fn new(fg: Option<Color>, bg: Option<Color>) -> Self {
        ColorPair { fg, bg }
    }
}

/// A precomputed sequence of colors interpolated across one or more stops.
#[derive(Debug, Clone)]
pub struct Gradient {
    pub spectrum: Vec<Color>,
}

impl Gradient {
    /// Build a gradient spectrum of `steps` colors per segment between
    /// consecutive `stops`. Mirrors the shape of `graphics.Gradient`, minus
    /// direction/canvas-mapping helpers (not needed by the skeleton yet).
    pub fn new(stops: &[Color], steps: usize) -> Self {
        let steps = steps.max(1);
        let mut spectrum = Vec::new();
        if stops.is_empty() {
            return Gradient { spectrum };
        }
        if stops.len() == 1 {
            spectrum.push(stops[0]);
            return Gradient { spectrum };
        }
        for pair in stops.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            for i in 0..steps {
                let t = i as f64 / steps as f64;
                spectrum.push(Color::lerp(a, b, t));
            }
        }
        spectrum.push(*stops.last().unwrap());
        Gradient { spectrum }
    }

    pub fn get(&self, index: usize) -> Option<Color> {
        self.spectrum.get(index).copied()
    }

    pub fn len(&self) -> usize {
        self.spectrum.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spectrum.is_empty()
    }
}
