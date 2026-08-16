//! Colors, color pairs, and gradients.

/// A 24-bit RGB color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }

    /// Parse `#rrggbb` or `rrggbb`.
    pub fn from_hex(hex: &str) -> Option<Color> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Color { r, g, b })
    }

    pub fn to_ansi_fg(&self) -> String {
        format!("\x1b[38;2;{};{};{}m", self.r, self.g, self.b)
    }

    pub fn to_ansi_bg(&self) -> String {
        format!("\x1b[48;2;{};{};{}m", self.r, self.g, self.b)
    }
}

/// Optional foreground/background pair.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ColorPair {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
}

impl ColorPair {
    pub fn new(fg: Option<Color>, bg: Option<Color>) -> Self {
        ColorPair { fg, bg }
    }

    pub fn fg(color: Color) -> Self {
        ColorPair {
            fg: Some(color),
            bg: None,
        }
    }
}

fn lerp_channel(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t).round().clamp(0.0, 255.0) as u8
}

/// A spectrum of colors interpolated between stops.
#[derive(Clone, Debug, Default)]
pub struct Gradient {
    pub spectrum: Vec<Color>,
}

impl Gradient {
    /// Build a gradient with `steps` interpolated colors between each pair of stops.
    pub fn new(stops: &[Color], steps: usize) -> Self {
        let mut spectrum = Vec::new();
        match stops {
            [] => {}
            [only] => spectrum.push(*only),
            _ => {
                let steps = steps.max(1);
                spectrum.push(stops[0]);
                for pair in stops.windows(2) {
                    let (a, b) = (pair[0], pair[1]);
                    for s in 1..=steps {
                        let t = s as f64 / steps as f64;
                        spectrum.push(Color::new(
                            lerp_channel(a.r, b.r, t),
                            lerp_channel(a.g, b.g, t),
                            lerp_channel(a.b, b.b, t),
                        ));
                    }
                }
            }
        }
        Gradient { spectrum }
    }

    /// Color at `fraction` (0..=1) along the spectrum.
    pub fn get_color_at_fraction(&self, fraction: f64) -> Option<Color> {
        if self.spectrum.is_empty() {
            return None;
        }
        let fraction = fraction.clamp(0.0, 1.0);
        let index = (fraction * (self.spectrum.len() - 1) as f64).round() as usize;
        Some(self.spectrum[index])
    }
}
