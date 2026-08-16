use crate::engine::character::EffectCharacter;
use crate::utils::geometry::Coord;
use crate::utils::graphics::Style;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub symbol: char,
    pub style: Style,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            symbol: ' ',
            style: Style::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Canvas {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    blank: Cell,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let blank = Cell::default();

        Self {
            width,
            height,
            cells: vec![blank.clone(); width * height],
            blank,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn contains(&self, coord: Coord) -> bool {
        coord.x >= 0
            && coord.y >= 0
            && (coord.x as usize) < self.width
            && (coord.y as usize) < self.height
    }

    pub fn get(&self, coord: Coord) -> Option<&Cell> {
        self.index(coord).map(|index| &self.cells[index])
    }

    pub fn get_mut(&mut self, coord: Coord) -> Option<&mut Cell> {
        let index = self.index(coord)?;
        Some(&mut self.cells[index])
    }

    pub fn set(&mut self, coord: Coord, cell: Cell) -> bool {
        let Some(index) = self.index(coord) else {
            return false;
        };

        self.cells[index] = cell;
        true
    }

    pub fn set_symbol(&mut self, coord: Coord, symbol: char) -> bool {
        let Some(cell) = self.get_mut(coord) else {
            return false;
        };

        cell.symbol = symbol;
        true
    }

    pub fn clear(&mut self) {
        self.cells.fill(self.blank.clone());
    }

    pub fn clear_with(&mut self, cell: Cell) {
        self.blank = cell;
        self.clear();
    }

    pub fn draw_character(&mut self, character: &EffectCharacter) -> bool {
        if !character.visible {
            return false;
        }

        self.set(
            character.position,
            Cell {
                symbol: character.symbol,
                style: character.style.clone(),
            },
        )
    }

    pub fn render(&self) -> String {
        let mut output = String::new();
        let mut active_style = Style::default();

        for y in 0..self.height {
            for x in 0..self.width {
                let cell = &self.cells[y * self.width + x];

                if cell.style != active_style {
                    if !active_style.is_default() {
                        output.push_str("\x1b[0m");
                    }

                    output.push_str(&cell.style.ansi_prefix());
                    active_style = cell.style.clone();
                }

                output.push(cell.symbol);
            }

            if !active_style.is_default() {
                output.push_str("\x1b[0m");
                active_style = Style::default();
            }

            if y + 1 < self.height {
                output.push('\n');
            }
        }

        output
    }

    fn index(&self, coord: Coord) -> Option<usize> {
        if !self.contains(coord) {
            return None;
        }

        Some(coord.y as usize * self.width + coord.x as usize)
    }
}
