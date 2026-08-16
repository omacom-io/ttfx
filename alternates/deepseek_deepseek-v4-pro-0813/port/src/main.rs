use clap::Parser;
use std::io::Read;

#[derive(Parser, Debug)]
#[command(name = "ttfx", version, about = "Terminal text effects in Rust")]
struct Args {
    /// Effect name to run
    effect: Option<String>,

    /// Text to animate. Reads stdin when omitted.
    #[arg(short, long)]
    text: Option<String>,
}

fn main() {
    let args = Args::parse();

    let input = match args.text {
        Some(text) => text,
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .expect("failed to read stdin");
            buf
        }
    };

    let effects = ttfx::effects::registry();

    if effects.is_empty() {
        eprintln!("No effects are registered.");
        std::process::exit(1);
    }

    let effect_name = args.effect.unwrap_or_else(|| effects[0].name().to_owned());
    let effect = effects
        .into_iter()
        .find(|e| e.name() == effect_name.as_str())
        .unwrap_or_else(|| {
            eprintln!("unknown effect: {effect_name}");
            std::process::exit(1);
        });

    for frame in effect.frames(&input) {
        println!("{frame}");
    }
}
