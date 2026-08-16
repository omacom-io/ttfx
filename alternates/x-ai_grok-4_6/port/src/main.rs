use std::io::{self, IsTerminal, Read, Write};
use std::thread;
use std::time::Duration;

use clap::{CommandFactory, Parser};
use crossterm::{cursor, execute};

use ttfx::effects::registry;
use ttfx::engine::terminal::{Terminal, TerminalConfig};

#[derive(Parser, Debug)]
#[command(
    name = "ttfx",
    version,
    about = "Terminal Text Effects — Rust port of TTE"
)]
struct Cli {
    /// Effect to run (see --list)
    effect: Option<String>,

    /// List registered effects and exit
    #[arg(short, long)]
    list: bool,

    /// Frames per second when playing an effect
    #[arg(long, default_value_t = 60)]
    frame_rate: u32,
}

fn main() {
    let cli = Cli::parse();
    let effects = registry();

    if cli.list {
        if effects.is_empty() {
            println!("(no effects registered)");
        } else {
            for effect in &effects {
                println!("{}", effect.name());
            }
        }
        return;
    }

    if cli.effect.is_none() && io::stdin().is_terminal() {
        let _ = Cli::command().print_help();
        println!();
        return;
    }

    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);

    let Some(name) = cli.effect.as_deref() else {
        let mut term = Terminal::from_input(&input, TerminalConfig::default());
        term.show_all();
        print!("{}", term.render_frame());
        return;
    };

    let Some(effect) = effects.iter().find(|effect| effect.name() == name) else {
        eprintln!("ttfx: unknown effect '{name}'");
        std::process::exit(2);
    };

    let frames = effect.frames(&input);
    let delay = if cli.frame_rate == 0 {
        Duration::ZERO
    } else {
        Duration::from_millis(1000 / u64::from(cli.frame_rate))
    };

    let mut stdout = io::stdout();
    let interactive = stdout.is_terminal();
    if interactive {
        let _ = execute!(stdout, cursor::Hide);
    }

    for (index, frame) in frames.iter().enumerate() {
        if interactive && index > 0 {
            let _ = execute!(stdout, cursor::MoveTo(0, 0));
        }
        print!("{frame}");
        let _ = stdout.flush();
        if delay > Duration::ZERO && index + 1 < frames.len() {
            thread::sleep(delay);
        }
    }

    if interactive {
        let _ = execute!(stdout, cursor::Show);
    }
}
