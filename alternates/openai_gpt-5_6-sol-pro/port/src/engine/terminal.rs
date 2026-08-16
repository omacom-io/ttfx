use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use crossterm::cursor::MoveTo;
use crossterm::terminal::{Clear, ClearType};
use crossterm::QueueableCommand;

use crate::engine::canvas::Canvas;
use crate::engine::character::{CharacterId, EffectCharacter};
use crate::utils::geometry::Coord;

#[derive(Debug, Clone)]
pub struct Terminal {
    canvas: Canvas,
    characters: Vec<EffectCharacter>,
    next_character_id: u32,
}

impl Terminal {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            canvas: Canvas::new(width, height),
            characters: Vec::new(),
            next_character_id: 0,
        }
    }

    pub fn from_text(input: &str) -> Self {
        let lines: Vec<&str> = if input.is_empty() {
            vec![""]
        } else {
            input.lines().collect()
        };

        let width = lines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0)
            .max(1);
        let height = lines.len().max(1);

        let mut terminal = Self::new(width, height);

        for (y, line) in lines.iter().enumerate() {
            for (x, symbol) in line.chars().enumerate() {
                terminal.add_character(symbol, Coord::new(x as i32, y as i32));
            }
        }

        terminal
    }

    pub fn canvas(&self) -> &Canvas {
        &self.canvas
    }

    pub fn canvas_mut(&mut self) -> &mut Canvas {
        &mut self.canvas
    }

    pub fn characters(&self) -> &[EffectCharacter] {
        &self.characters
    }

    pub fn characters_mut(&mut self) -> &mut [EffectCharacter] {
        &mut self.characters
    }

    pub fn character(&self, id: CharacterId) -> Option<&EffectCharacter> {
        self.characters
            .iter()
            .find(|character| character.id == id)
    }

    pub fn character_mut(&mut self, id: CharacterId) -> Option<&mut EffectCharacter> {
        self.characters
            .iter_mut()
            .find(|character| character.id == id)
    }

    pub fn add_character(&mut self, symbol: char, position: Coord) -> CharacterId {
        let id = CharacterId(self.next_character_id);
        self.next_character_id = self.next_character_id.saturating_add(1);

        self.characters
            .push(EffectCharacter::new(id, symbol, position));

        id
    }

    pub fn remove_character(&mut self, id: CharacterId) -> Option<EffectCharacter> {
        let index = self
            .characters
            .iter()
            .position(|character| character.id == id)?;

        Some(self.characters.remove(index))
    }

    pub fn step(&mut self) {
        for character in &mut self.characters {
            character.step();
        }
    }

    pub fn render_frame(&mut self) -> String {
        self.canvas.clear();

        for character in &self.characters {
            self.canvas.draw_character(character);
        }

        self.canvas.render()
    }

    pub fn run_steps<F>(&mut self, max_steps: usize, mut continue_running: F) -> Vec<String>
    where
        F: FnMut(&mut Self, usize) -> bool,
    {
        let mut frames = Vec::new();

        for step in 0..max_steps {
            if !continue_running(self, step) {
                break;
            }

            self.step();
            frames.push(self.render_frame());
        }

        frames
    }

    pub fn play_frames<'a, W, I>(
        &self,
        output: &mut W,
        frames: I,
        frame_delay: Duration,
    ) -> io::Result<()>
    where
        W: Write,
        I: IntoIterator<Item = &'a str>,
    {
        let frames: Vec<&str> = frames.into_iter().collect();

        for (index, frame) in frames.iter().enumerate() {
            if index > 0 {
                output.queue(MoveTo(0, 0))?;
                output.queue(Clear(ClearType::All))?;
            }

            output.write_all(frame.as_bytes())?;
            output.flush()?;

            if index + 1 < frames.len() && !frame_delay.is_zero() {
                thread::sleep(frame_delay);
            }
        }

        if !frames.is_empty() {
            output.write_all(b"\n")?;
            output.flush()?;
        }

        Ok(())
    }
}
