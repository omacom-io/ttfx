use std::io::{IsTerminal, Read};
use std::process::ExitCode;

use clap::Parser;

use ttfx::engine::terminal::Terminal;
use ttfx::{cli, engine};

fn get_piped_input() -> String {
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        return String::new();
    }
    let mut buf: Vec<u8> = Vec::new();
    if stdin.lock().read_to_end(&mut buf).is_err() {
        return String::new();
    }
    // strict UTF-8, like Python's text-mode stdin (plan.md §8)
    match String::from_utf8(buf) {
        Ok(s) => s,
        Err(e) => {
            println!("Error decoding input: {e}");
            std::process::exit(1);
        }
    }
}

fn main() -> ExitCode {
    let cli = cli::Cli::parse();

    let input_data = match &cli.input_file {
        Some(path) => match std::fs::read(path) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(s) => s,
                Err(e) => {
                    // upstream prints runtime file errors to STDOUT and exits 1
                    println!("Error reading input file: {e}");
                    return ExitCode::from(1);
                }
            },
            Err(e) => {
                println!("Error reading input file: {e}");
                return ExitCode::from(1);
            }
        },
        None => get_piped_input(),
    };

    if input_data.trim().is_empty() {
        println!("NO INPUT.");
        return ExitCode::from(1);
    }

    if cli.m0_dump {
        return m0_dump(&input_data, &cli);
    }

    // Effect dispatch lands in M3; until then only the debug path exists.
    eprintln!("no effect specified (effects arrive in a later milestone)");
    ExitCode::from(1)
}

/// M0 parity path: build the Terminal, make every character in
/// character_by_input_coord visible, print the first frame to stdout.
fn m0_dump(input_data: &str, cli: &cli::Cli) -> ExitCode {
    let config = cli.terminal_config();
    let mut terminal = match Terminal::new(input_data, config) {
        Ok(t) => t,
        Err(engine::error::EngineError::UnsupportedAnsiSequence(seq)) => {
            eprintln!("Error: Unsupported ANSI sequence in input data: {seq:?}");
            return ExitCode::from(1);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::from(1);
        }
    };
    let ids: Vec<_> = terminal.character_by_input_coord.values().copied().collect();
    for id in ids {
        terminal.set_character_visibility(id, true);
    }
    print!("{}", terminal.get_formatted_output_string());
    println!();
    ExitCode::SUCCESS
}
