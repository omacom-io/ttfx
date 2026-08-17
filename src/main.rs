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
            ttfx::outln!("Error decoding input: {e}");
            std::process::exit(1);
        }
    }
}

/// Skip the arena teardown on the way out. Output is already flushed and
/// nothing in the engine has a Drop impl that does work, so freeing tens of
/// thousands of characters — each with its own scenes, paths and frames — one
/// by one is pure exit latency; on binarypath it is ~4% of the run.
///
/// Only the exit paths come through here. A resize rebuild drops its engine
/// normally, so a long session of resizes does not accumulate them.
fn forget_engine<E, C>(effect: E, ctx: C) {
    std::mem::forget(effect);
    std::mem::forget(ctx);
}

fn main() -> ExitCode {
    ttfx::restore_sigpipe();
    let cli = cli::Cli::parse();

    // upstream prints the completion script and returns before any input handling
    if let Some(shell) = &cli.print_completion {
        use clap::CommandFactory;
        let generator = match shell.as_str() {
            "bash" => clap_complete::Shell::Bash,
            _ => clap_complete::Shell::Zsh,
        };
        let mut command = cli::Cli::command();
        clap_complete::generate(generator, &mut command, "ttfx", &mut std::io::stdout());
        return ExitCode::SUCCESS;
    }

    let input_data = match &cli.input_file {
        Some(path) => match std::fs::read(path) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(s) => s,
                Err(e) => {
                    // upstream prints runtime file errors to STDOUT and exits 1
                    ttfx::outln!("Error reading input file: {e}");
                    return ExitCode::from(1);
                }
            },
            Err(e) => {
                ttfx::outln!("Error reading input file: {e}");
                return ExitCode::from(1);
            }
        },
        None => get_piped_input(),
    };

    if input_data.trim().is_empty() {
        ttfx::outln!("NO INPUT.");
        return ExitCode::from(1);
    }

    if cli.m0_dump {
        return m0_dump(&input_data, &cli);
    }

    let mut rng = match cli.seed {
        Some(seed) => ttfx::utils::rng::Rng::seeded(seed),
        None => ttfx::utils::rng::Rng::from_entropy(),
    };

    // --random-effect draws from the registry (filtered) and runs with pure
    // default effect config — upstream ignores effect CLI args here too. The
    // pool is resolved once; the draw itself happens per pass, so a --loop run
    // cycles effects instead of repeating one.
    let random_pool = if cli.random_effect {
        match ttfx::effects::filtered_names(&cli.include_effects, &cli.exclude_effects) {
            Ok(names) if names.is_empty() => {
                ttfx::errln!("Error: No effects available after filtering.");
                return ExitCode::from(1);
            }
            Ok(names) => names,
            Err(unknown) => {
                ttfx::errln!("Error: No effect named '{unknown}'.");
                return ExitCode::from(1);
            }
        }
    } else {
        if cli.effect.is_none() {
            ttfx::errln!("Error: No effect specified.");
            return ExitCode::from(1);
        }
        Vec::new()
    };

    let mut config = cli.terminal_config();
    // SIGWINCH is delivered to every process in the terminal's foreground group,
    // whatever its stdout points at. Reacting to it when the animation is being
    // redirected would leave a truncated first run followed by a complete second
    // one in the file. SIGTERM teardown is tty-only for the same reason: a
    // redirected stream must not gain teardown bytes. Only the teardown differs
    // — the tty run re-raises afterwards, so both die from the signal.
    let tty_output = !cli.parity_dump && std::io::stdout().is_terminal();
    if !cli.parity_dump {
        ttfx::install_sigint_handler();
    }
    if tty_output {
        ttfx::install_sigterm_handler();
        ttfx::install_sigwinch_handler();
    }

    // --loop keeps drawing until a signal ends the run. A parity dump is a
    // fixed-length artifact, so it never repeats.
    let repeating = cli.loop_forever && !cli.parity_dump;

    let result = loop {
        let drawn_effect;
        let effect_command = if cli.random_effect {
            let name = random_pool[rng.choice_index(random_pool.len())];
            match ttfx::effects::EffectCommand::from_name(name) {
                Some(command) => {
                    drawn_effect = command;
                    &drawn_effect
                }
                None => {
                    ttfx::errln!("Error: failed to build effect '{name}'.");
                    return ExitCode::from(1);
                }
            }
        } else {
            cli.effect.as_ref().expect("a missing effect already exited")
        };

        let clock = if cli.parity_dump || cli.virtual_clock {
            ttfx::engine::ctx::Clock::virtual_with_frame_rate(config.frame_rate)
        } else {
            ttfx::engine::ctx::Clock::real()
        };
        let mut ctx = match ttfx::engine::ctx::EngineCtx::new(
            &input_data,
            config.clone(),
            rng,
            clock,
        ) {
            Ok(ctx) => ctx,
            Err(engine::error::EngineError::UnsupportedAnsiSequence(seq)) => {
                ttfx::errln!("Error: Unsupported ANSI sequence in input data: {seq:?}");
                return ExitCode::from(1);
            }
            Err(e) => {
                ttfx::errln!("Error: {e}");
                return ExitCode::from(1);
            }
        };
        let mut effect = effect_command.build_effect();

        let outcome = if cli.parity_dump {
            ttfx::engine::effect::dump_effect(effect.as_mut(), &mut ctx, cli.max_frames)
                .map(|_| ttfx::engine::effect::RunOutcome::Complete)
        } else {
            ttfx::engine::effect::run_effect(effect.as_mut(), &mut ctx, tty_output, repeating)
        };
        match outcome {
            Ok(ttfx::engine::effect::RunOutcome::TerminalResized) => {
                // run_effect wiped the old area and left the cursor at its top,
                // so the rebuild lays out from here. --reuse-canvas would send
                // prep_canvas to a DEC anchor that no longer applies, so it only
                // governs the first run. Dropping this engine normally is what
                // keeps a long session of resizes from accumulating them.
                config.reuse_canvas = false;
                rng = ctx.rng;
            }
            Ok(ttfx::engine::effect::RunOutcome::Complete) if repeating => {
                // A repeating pass tore down exactly like a resize, so the next
                // one lays out in the wiped area for the same reason. Carrying
                // the RNG forward is what keeps a --seed run deterministic
                // across the whole sequence rather than only its first pass.
                config.reuse_canvas = false;
                rng = ctx.rng;
            }
            done => {
                forget_engine(effect, ctx);
                break done.map(|_| ());
            }
        }
    };
    std::mem::forget(input_data);

    match result {
        Ok(()) => {
            if ttfx::terminated() {
                ttfx::die_from_sigterm();
            }
            if ttfx::interrupted() {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            ttfx::errln!("Error: {e}");
            ExitCode::from(1)
        }
    }
}

/// M0 parity path: build the Terminal, make every character in
/// character_by_input_coord visible, print the first frame to stdout.
fn m0_dump(input_data: &str, cli: &cli::Cli) -> ExitCode {
    let config = cli.terminal_config();
    let mut terminal = match Terminal::new(input_data, config) {
        Ok(t) => t,
        Err(engine::error::EngineError::UnsupportedAnsiSequence(seq)) => {
            ttfx::errln!("Error: Unsupported ANSI sequence in input data: {seq:?}");
            return ExitCode::from(1);
        }
        Err(e) => {
            ttfx::errln!("Error: {e}");
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
