#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    Rgb { r: u8, g: u8, b: u8 },
    Ansi(u8),
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::Rgb { r, g, b }
    }

    pub const fn ansi(value: u8) -> Self {
        Self::Ansi(value)
    }

    fn foreground_code(self) -> String {
        match self {
            Self::Rgb { r, g, b } => format!("38;2;{r};{g};{b}"),
            Self::Ansi(value) => format!("38;5;{value}"),
        }
    }

    fn background_code(self) -> String {
        match self {
            Self::Rgb { r, g, b } => format!("48;2;{r};{g};{b}"),
            Self::Ansi(value) => format!("48;5;{value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ColorPair {
    pub foreground: Option<Color>,
    pub background: Option<Color>,
}

impl ColorPair {
    pub const fn new(foreground: Option<Color>, background: Option<Color>) -> Self {
        Self {
            foreground,
            background,
        }
    }

    pub const fn foreground(color: Color) -> Self {
        Self::new(Some(color), None)
    }

    pub const fn background(color: Color) -> Self {
        Self::new(None, Some(color))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Style {
    pub colors: ColorPair,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub reverse: bool,
    pub hidden: bool,
    pub strike: bool,
}

impl Style {
    pub fn with_foreground(mut self, color: Color) -> Self {
        self.colors.foreground = Some(color);
        self
    }

    pub fn with_background(mut self, color: Color) -> Self {
        self.colors.background = Some(color);
        self
    }

    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }

    pub fn ansi_prefix(&self) -> String {
        let mut codes = Vec::new();

        if self.bold {
            codes.push("1".to_owned());
        }
        if self.dim {
            codes.push("2".to_owned());
        }
        if self.italic {
            codes.push("3".to_owned());
        }
        if self.underline {
            codes.push("4".to_owned());
        }
        if self.blink {
            codes.push("5".to_owned());
        }
        if self.reverse {
            codes.push("7".to_owned());
        }
        if self.hidden {
            codes.push("8".to_owned());
        }
        if self.strike {
            codes.push("9".to_owned());
        }

        if let Some(foreground) = self.colors.foreground {
            codes.push(foreground.foreground_code());
        }

        if let Some(background) = self.colors.background {
            codes.push(background.background_code());
        }

        if codes.is_empty() {
            String::new()
        } else {
            format!("\x1b[{}m", codes.join(";"))
        }
    }
}
