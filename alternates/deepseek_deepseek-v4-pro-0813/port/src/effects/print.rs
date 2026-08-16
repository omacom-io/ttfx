use super::Effect;
use crate::engine::terminal::Terminal;

pub struct Print;

impl Print {
    pub fn new() -> Self {
        Self
    }
}

impl Effect for Print {
    fn name(&self) -> &str {
        "print"
    }

    fn frames(&self, input: &str) -> Vec<String> {
        let (width, height) = Terminal::autodetect_size();
        let terminal = Terminal::from_input(input, width, height);
        vec![terminal.write_frame()]
    }
}
