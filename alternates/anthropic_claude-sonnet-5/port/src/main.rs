//! CLI entry point: read stdin, pick an effect by name from the registry,
//! and run it. Falls back to an identity render (no animation) when no
//! effect is selected or found, so the skeleton binary is always usable
//! (mirrors the M0 exit criterion in plan.md: piping text through a no-op
//! effect reproduces the preprocessed first frame).

use std::io::{self, Read};

use clap::Parser;

use ttfx::effects;
use ttfx::engine::terminal::Terminal;

#[derive(Parser, Debug)]
#[command(name = "ttfx", about = "Rust port of TerminalTextEffects (core skeleton)")]
struct Cli {
    /// Name of the effect to run (e.g. "rain", "beams"). If omitted or
    /// unrecognized, input is rendered as-is.
    effect: Option<String>,

    /// List available effect names and exit.
    #[arg(long)]
    list: bool,
}

fn read_stdin() -> String {
    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_err() {
        buf.clear();
    }
    buf
}

fn main() {
    let cli = Cli::parse();

    // Establish a terminal-size fallback the same way the Python CLI does
    // (80x24 on failure), consistent with plan.md's crossterm rationale.
    let _terminal_size = crossterm::terminal::size().unwrap_or((80, 24));

    let available = effects::registry();

    if cli.list {
        if available.is_empty() {
            println!("(no effects registered yet)");
        } else {
            for effect in &available {
                println!("{}", effect.name());
            }
        }
        return;
    }

    let input = read_stdin();

    if let Some(name) = &cli.effect {
        if let Some(effect) = available.iter().find(|e| e.name() == name) {
            for frame in effect.frames(&input) {
                println!("{frame}");
            }
            return;
        }
        eprintln!("ttfx: unknown effect '{name}', falling back to identity render");
    }

    // Identity fallback: build the terminal/canvas/character arena from the
    // input and render the single unanimated frame.
    let terminal = Terminal::new(&input);
    println!("{}", terminal.render());
}
