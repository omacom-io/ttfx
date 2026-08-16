use std::io::{self, Read, Write};
use std::thread;
use std::time::Duration;

use clap::Parser;
use crossterm::{cursor, execute, terminal};

/// ttfx — terminal text effects (Rust port of TTE).
#[derive(Parser)]
#[command(name = "ttfx", version, about = "Apply visual effects to text piped over stdin.")]
struct Cli {
    /// Name of the effect to run.
    effect: Option<String>,

    /// Frames per second used when playing back effect frames.
    #[arg(long, default_value_t = 60)]
    frame_rate: u64,

    /// List available effects and exit.
    #[arg(long)]
    list: bool,
}

fn main() {
    let cli = Cli::parse();
    let effects = ttfx::effects::registry();

    if cli.list || cli.effect.is_none() {
        println!("available effects:");
        if effects.is_empty() {
            println!("  (none registered yet)");
        }
        for effect in &effects {
            println!("  {}", effect.name());
        }
        return;
    }

    let name = cli.effect.unwrap();

    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() || input.trim().is_empty() {
        input = String::from("ttfx");
    }

    match effects.iter().find(|e| e.name() == name) {
        Some(effect) => {
            let frames = effect.frames(&input);
            if let Err(err) = play(&frames, cli.frame_rate) {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
        None => {
            eprintln!("unknown effect: {name}");
            eprintln!("run with --list to see available effects");
            std::process::exit(1);
        }
    }
}

fn play(frames: &[String], frame_rate: u64) -> io::Result<()> {
    let mut out = io::stdout();
    let delay = Duration::from_millis(1000 / frame_rate.max(1));

    execute!(out, cursor::Hide)?;
    for frame in frames {
        execute!(
            out,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0)
        )?;
        write!(out, "{frame}")?;
        out.flush()?;
        thread::sleep(delay);
    }
    execute!(out, cursor::Show)?;
    writeln!(out)?;
    Ok(())
}
