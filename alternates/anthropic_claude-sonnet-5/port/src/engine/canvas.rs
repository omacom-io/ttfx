//! Canvas: a grid of fill cells backing the input text (mirrors the
//! dimension/anchoring role of terminaltexteffects.engine.terminal.Canvas,
//! simplified to a plain character grid for the skeleton).

#[derive(Debug, Clone, Copy)]
pub struct Cell {
    pub symbol: char,
}

#[derive(Debug, Clone)]
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<Cell>>,
}

impl Canvas {
    /// Build a canvas from raw input text, one row per line, width equal to
    /// the longest line (short lines padded with spaces), matching the
    /// upstream behavior of sizing the canvas to the input text bounds.
    pub fn from_text(input: &str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        let height = lines.len().max(1);
        let width = lines
            .iter()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0)
            .max(1);

        let mut cells = Vec::with_capacity(height);
        for row_idx in 0..height {
            let line_chars: Vec<char> = lines.get(row_idx).map(|l| l.chars().collect()).unwrap_or_default();
            let mut row = Vec::with_capacity(width);
            for col_idx in 0..width {
                let symbol = *line_chars.get(col_idx).unwrap_or(&' ');
                row.push(Cell { symbol });
            }
            cells.push(row);
        }
        Canvas { width, height, cells }
    }

    pub fn get(&self, column: usize, row: usize) -> Option<char> {
        self.cells.get(row).and_then(|r| r.get(column)).map(|c| c.symbol)
    }
}
