use crate::utils::{floor_div, round_half_even};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let h = hex.trim().trim_start_matches('#');
        match h.len() {
            3 => {
                let r = u8::from_str_radix(&h[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&h[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&h[2..3].repeat(2), 16).ok()?;
                Some(Self { r, g, b })
            }
            6 => {
                let r = u8::from_str_radix(&h[0..2], 16).ok()?;
                let g = u8::from_str_radix(&h[2..4], 16).ok()?;
                let b = u8::from_str_radix(&h[4..6], 16).ok()?;
                Some(Self { r, g, b })
            }
            _ => None,
        }
    }

    pub fn from_xterm(idx: u8) -> Self {
        let (r, g, b) = xterm_to_rgb(idx);
        Self { r, g, b }
    }

    pub fn hex(&self) -> String {
        format!("{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn fg_sgr(&self) -> String {
        format!("\x1b[38;2;{};{};{}m", self.r, self.g, self.b)
    }

    pub fn bg_sgr(&self) -> String {
        format!("\x1b[48;2;{};{};{}m", self.r, self.g, self.b)
    }

    pub fn adjust_brightness(self, factor: f64) -> Self {
        let adj = |channel: u8| -> u8 {
            round_half_even(f64::from(channel) * factor).clamp(0, 255) as u8
        };
        Self {
            r: adj(self.r),
            g: adj(self.g),
            b: adj(self.b),
        }
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from(rgb: (u8, u8, u8)) -> Self {
        Self::rgb(rgb.0, rgb.1, rgb.2)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ColorPair {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
}

impl ColorPair {
    pub fn new(fg: Option<Color>, bg: Option<Color>) -> Self {
        Self { fg, bg }
    }

    pub fn fg(color: Color) -> Self {
        Self {
            fg: Some(color),
            bg: None,
        }
    }

    pub fn bg(color: Color) -> Self {
        Self {
            fg: None,
            bg: Some(color),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GradientDirection {
    #[default]
    Vertical,
    Horizontal,
    Diagonal,
    Radial,
    Center,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Gradient {
    spectrum: Vec<Color>,
}

impl Gradient {
    /// Channel deltas use Python integer floor division: `(end - start) // steps`.
    pub fn new(stops: &[Color], steps: usize) -> Self {
        if stops.is_empty() {
            return Self {
                spectrum: Vec::new(),
            };
        }
        if stops.len() == 1 || steps == 0 {
            return Self {
                spectrum: stops.to_vec(),
            };
        }
        let mut spectrum = Vec::new();
        let n = steps as i32;
        for pair in stops.windows(2) {
            let start = pair[0];
            let end = pair[1];
            let dr = floor_div(i32::from(end.r) - i32::from(start.r), n);
            let dg = floor_div(i32::from(end.g) - i32::from(start.g), n);
            let db = floor_div(i32::from(end.b) - i32::from(start.b), n);
            for i in 0..n {
                spectrum.push(Color {
                    r: (i32::from(start.r) + dr * i).clamp(0, 255) as u8,
                    g: (i32::from(start.g) + dg * i).clamp(0, 255) as u8,
                    b: (i32::from(start.b) + db * i).clamp(0, 255) as u8,
                });
            }
        }
        if let Some(last) = stops.last() {
            spectrum.push(*last);
        }
        Self { spectrum }
    }

    pub fn spectrum(&self) -> &[Color] {
        &self.spectrum
    }

    pub fn is_empty(&self) -> bool {
        self.spectrum.is_empty()
    }

    pub fn len(&self) -> usize {
        self.spectrum.len()
    }

    pub fn get(&self, index: usize) -> Option<Color> {
        self.spectrum.get(index).copied()
    }

    pub fn mapped_color(&self, progress: f64) -> Option<Color> {
        if self.spectrum.is_empty() {
            return None;
        }
        if self.spectrum.len() == 1 {
            return Some(self.spectrum[0]);
        }
        let last = (self.spectrum.len() - 1) as f64;
        let idx = round_half_even(progress.clamp(0.0, 1.0) * last).clamp(0, last as i64) as usize;
        Some(self.spectrum[idx])
    }
}

pub fn xterm_to_rgb(idx: u8) -> (u8, u8, u8) {
    const SYS: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (128, 0, 0),
        (0, 128, 0),
        (128, 128, 0),
        (0, 0, 128),
        (128, 0, 128),
        (0, 128, 128),
        (192, 192, 192),
        (128, 128, 128),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (0, 0, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    if idx < 16 {
        return SYS[idx as usize];
    }
    if idx < 232 {
        let n = idx - 16;
        let r = n / 36;
        let g = (n % 36) / 6;
        let b = n % 6;
        const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
        (
            LEVELS[r as usize],
            LEVELS[g as usize],
            LEVELS[b as usize],
        )
    } else {
        let v = 8 + 10 * (idx - 232);
        (v, v, v)
    }
}
