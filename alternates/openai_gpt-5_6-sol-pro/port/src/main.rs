use std::error::Error;
use std::io::{self, Read};
use std::time::Duration;

use clap::Parser;
use ttfx::effects;
use ttfx::engine::terminal::Terminal;

#[derive(Debug, Parser)]
#[command(
    name = "ttfx",
    version,
    about = "Terminal text effects implemented in Rust"
)]
struct Cli {
    /// Effect to run.
    effect: Option<String>,

    /// Delay between frames in milliseconds.
    #[arg(long, default_value_t = 50)]
    frame_delay: u64,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("ttfx: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let frames = if let Some(effect_name) = cli.effect.as_deref() {
        let effect = effects::registry()
            .into_iter()
            .find(|effect| effect.name() == effect_name)
            .ok_or_else(|| {
                let available = effects::registry()
                    .into_iter()
                    .map(|effect| effect.name().to_owned())
                    .collect::<Vec<_>>();

                if available.is_empty() {
                    format!(
                        "unknown effect '{effect_name}'; no effects are currently registered"
                    )
                } else {
                    format!(
                        "unknown effect '{effect_name}'; available effects: {}",
                        available.join(", ")
                    )
                }
            })?;

        effect.frames(&input)
    } else {
        let mut terminal = Terminal::from_text(&input);
        terminal.run_steps(1, |_terminal, _step| true)
    };

    let terminal = Terminal::new(1, 1);
    let stdout = io::stdout();
    let mut output = stdout.lock();

    terminal.play_frames(
        &mut output,
        frames.iter().map(String::as_str),
        Duration::from_millis(cli.frame_delay),
    )?;

    Ok(())
}
