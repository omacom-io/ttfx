
use crate::utils::graphics::{Color, ColorPair};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellStyle {
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub reverse: bool,
    pub hidden: bool,
}

impl CellStyle {
    pub fn new(fg: Color, bg: Color) -> Self {
        Self {
            fg,
            bg,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            reverse: false,
            hidden: false,
        }
    }

    pub fn with_color_pair(pair: ColorPair) -> Self {
        Self::new(pair.fg, pair.bg)
    }

    fn ansi_prefix(&self) -> String {
        let mut codes: Vec<String> = Vec::new();

        if self.bold {
            codes.push("1".to_string());
        }
        if self.dim {
            codes.push("2".to_string());
        }
        if self.italic {
            codes.push("3".to_string());
        }
        if self.underline {
            codes.push("4".to_string());
        }
        if self.reverse {
            codes.push("7".to_string());
        }
        if self.hidden {
            codes.push("8".to_string());
        }

        codes.push(format!("38;2;{};{};{}", self.fg.r, self.fg.g, self.fg.b));
        codes.push(format!("48;2;{};{};{}", self.bg.r, self.bg.g, self.bg.b));

        format!("\u{1b}[{}m", codes.join(";"))
    }

    pub fn render_symbol(&self, symbol: &str) -> String {
        format!("{}{}\u{1b}[0m", self.ansi_prefix(), symbol)
    }
}

impl Default for CellStyle {
    fn default() -> Self {
        Self::with_color_pair(ColorPair::default())
    }
}

#[derive(Clone, Debug)]
pub struct Cell {
    pub symbol: String,
    pub style: CellStyle,
}

impl Cell {
    pub fn new(symbol: impl Into<String>, style: CellStyle) -> Self {
        Self {
            symbol: symbol.into(),
            style,
        }
    }

    pub fn empty() -> Self {
        Self::new(" ", CellStyle::default())
    }
}

#[derive(Clone, Debug)]
pub struct Canvas {
    pub width: u16,
    pub height: u16,
    cells: Vec<Cell>,
}

impl Canvas {
    pub fn new(width: u16, height: u16) -> Self {
        let total = width as usize * height as usize;
        Self {
            width,
            height,
            cells: vec![Cell::empty(); total],
        }
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        if x < self.width && y < self.height {
            Some(y as usize * self.width as usize + x as usize)
        } else {
            None
        }
    }

    pub fn get(&self, x: u16, y: u16) -> Option<&Cell> {
        self.index(x, y).and_then(|i| self.cells.get(i))
    }

    pub fn get_mut(&mut self, x: u16, y: u16) -> Option<&mut Cell> {
        self.index(x, y).and_then(|i| self.cells.get_mut(i))
    }

    pub fn set_cell(&mut self, x: u16, y: u16, cell: Cell) {
        if let Some(i) = self.index(x, y) {
            self.cells[i] = cell;
        }
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = Cell::empty();
        }
    }

    pub fn render_frame(&self) -> String {
        let mut out = String::with_capacity(
            self.width as usize * self.height as usize * 32 + self.height as usize,
        );
        for y in 0..self.height {
            for x in 0..self.width {
                let cell = &self.cells[y as usize * self.width as usize + x as usize];
                out.push_str(&cell.style.render_symbol(&cell.symbol));
            }
            out.push('\n');
        }
        out
    }
}
