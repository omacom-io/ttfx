//! CLI root: global/terminal arguments, subcommand registry, and shell
//! completion generation. Mirrors `terminaltexteffects/__main__.py`'s
//! `build_parser()` plus `terminaltexteffects/utils/shell_completion.py`.

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Shells supported by `--print-completion`, matching upstream's
/// `SUPPORTED_SHELLS = ("bash", "zsh")`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CompletionShell {
    Bash,
    Zsh,
}

impl CompletionShell {
    /// The lowercase name as accepted on the command line.
    pub fn as_str(self) -> &'static str {
        match self {
            CompletionShell::Bash => "bash",
            CompletionShell::Zsh => "zsh",
        }
    }
}

/// Global / terminal-config arguments, populated ahead of the effect
/// subcommand (analog of `TerminalConfig._populate_parser`).
#[derive(Debug, Args, Clone, Default)]
pub struct TerminalArgs {
    /// Random seed for the pseudo-random number generator. Using the same seed
    /// will produce the same random effect decisions across runs.
    #[arg(long, value_name = "SEED", help = "Seed for the pseudo-random number generator")]
    pub seed: Option<u64>,

    /// Disable all colors in the effect.
    #[arg(long, help = "Disable all colors in the effect")]
    pub no_color: bool,

    /// Use xterm 256-color mode instead of the terminal's default color support.
    #[arg(long, help = "Use xterm 256 color mode instead of the terminal's default")]
    pub xterm_colors: bool,

    /// Frame rate, in frames per second.
    #[arg(long, default_value_t = 60, help = "Frame rate (frames per second)")]
    pub frame_rate: u32,

    /// Path to a file containing the input text (instead of stdin).
    #[arg(long, value_name = "FILE", help = "File to read input text from")]
    pub input_file: Option<String>,

    /// Randomly select an effect to run rather than requiring a subcommand.
    #[arg(long, help = "Select a random effect to run")]
    pub random_effect: bool,

    /// Space-separated list of effects to include when randomly selecting an effect.
    #[arg(
        long,
        num_args = 1..,
        value_name = "EFFECT",
        help = "Space-separated list of Effects to include when randomly selecting an effect"
    )]
    pub include_effects: Vec<String>,

    /// Space-separated list of effects to exclude when randomly selecting an effect.
    #[arg(
        long,
        num_args = 1..,
        value_name = "EFFECT",
        help = "Space-separated list of Effects to exclude when randomly selecting an effect"
    )]
    pub exclude_effects: Vec<String>,

    /// Print a shell completion script for the given shell and exit.
    #[arg(long, value_enum, value_name = "SHELL", help = "Print a shell completion script and exit")]
    pub print_completion: Option<CompletionShell>,
}

/// The effect to run is dispatched as an external subcommand: the concrete
/// per-effect argument parsing lives in each effect's own `clap::Command`
/// (built by `effects::mod`'s static registry), not here. This mirrors
/// `argparse`'s dynamic subparser registration in upstream's `build_parser`.
#[derive(Debug, Subcommand, Clone)]
pub enum EffectCommand {
    #[command(external_subcommand)]
    External(Vec<String>),
}

/// Root CLI definition: global/terminal args plus the effect subcommand.
#[derive(Debug, Parser, Clone)]
#[command(
    name = "ttfx",
    version,
    about = "Apply visual effects to terminal text",
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(flatten)]
    pub terminal: TerminalArgs,

    #[command(subcommand)]
    pub effect: Option<EffectCommand>,
}

impl Cli {
    /// Parse the process's `argv`, matching upstream's
    /// `build_parsers_and_parse_args()` entry point.
    pub fn parse_args() -> Self {
        Cli::parse()
    }

    /// The effect command name, if one was given (the analog of
    /// `args.effect` on the `argparse.Namespace`).
    pub fn effect_name(&self) -> Option<&str> {
        match &self.effect {
            Some(EffectCommand::External(argv)) => argv.first().map(String::as_str),
            None => None,
        }
    }

    /// The raw argv tokens following the effect name, to be re-parsed by the
    /// selected effect's own `clap::Command`.
    pub fn effect_args(&self) -> &[String] {
        match &self.effect {
            Some(EffectCommand::External(argv)) => {
                if argv.is_empty() {
                    &[]
                } else {
                    &argv[1..]
                }
            }
            None => &[],
        }
    }
}

/// Build the underlying `clap::Command` for the root parser. Kept separate
/// from `Cli` so completion generation can walk the command model without
/// needing a fully parsed `Cli` instance.
pub fn build_command() -> clap::Command {
    Cli::command()
}

// Re-export the `CommandFactory` trait usage without requiring callers to
// import clap directly.
use clap::CommandFactory;

/// Generate a static shell completion script for the requested shell,
/// mirroring `terminaltexteffects/utils/shell_completion.py`.
///
/// `effect_names` lists the registered effect subcommands (from
/// `effects::mod`'s static registry) so they can be offered as completions
/// alongside the global options.
pub fn get_completion_script(shell: CompletionShell, effect_names: &[&str]) -> String {
    match shell {
        CompletionShell::Bash => build_bash_completion(effect_names),
        CompletionShell::Zsh => build_zsh_completion(effect_names),
    }
}

fn escape_for_shell<'a, I: IntoIterator<Item = &'a str>>(words: I) -> String {
    words
        .into_iter()
        .map(|word| word.replace('\\', "\\\\").replace('"', "\\\""))
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_bash_completion(effect_names: &[&str]) -> String {
    let effects = escape_for_shell(effect_names.iter().copied());
    let shells = escape_for_shell(SUPPORTED_SHELLS.iter().copied());
    format!(
        r#"_ttfx_completion() {{
    local cur prev words cword
    _init_completion || return

    local effects="{effects}"
    local shells="{shells}"

    case "$prev" in
        --print-completion)
            COMPREPLY=($(compgen -W "$shells" -- "$cur"))
            return
            ;;
        --input-file)
            COMPREPLY=($(compgen -f -- "$cur"))
            return
            ;;
    esac

    if [[ ${{COMP_CWORD}} -eq 1 ]]; then
        COMPREPLY=($(compgen -W "$effects" -- "$cur"))
        return
    fi

    if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "--seed --no-color --xterm-colors --frame-rate --input-file --random-effect --include-effects --exclude-effects --print-completion --help --version" -- "$cur"))
        return
    fi
}}
complete -F _ttfx_completion ttfx
complete -F _ttfx_completion terminaltexteffects
"#
    )
}

fn build_zsh_completion(effect_names: &[&str]) -> String {
    let effects = escape_for_shell(effect_names.iter().copied());
    let shells = escape_for_shell(SUPPORTED_SHELLS.iter().copied());
    format!(
        r#"#compdef ttfx terminaltexteffects

_ttfx() {{
    local -a effects shells
    effects=({effects})
    shells=({shells})

    if (( CURRENT == 2 )); then
        _describe 'effect' effects
        return
    fi

    case "${{words[CURRENT-1]}}" in
        --print-completion)
            _describe 'shell' shells
            return
            ;;
        --input-file)
            _files
            return
            ;;
    esac

    _arguments \
        '--seed[seed for the pseudo-random number generator]' \
        '--no-color[disable all colors]' \
        '--xterm-colors[use xterm 256 color mode]' \
        '--frame-rate[frames per second]' \
        '--input-file[file to read input text from]:file:_files' \
        '--random-effect[select a random effect]' \
        '--include-effects[effects to include when randomly selecting]' \
        '--exclude-effects[effects to exclude when randomly selecting]' \
        '--print-completion[print a shell completion script]:shell:({shells})' \
        '--help[show help]' \
        '--version[show version]'
}}

compdef _ttfx ttfx
compdef _ttfx terminaltexteffects
"#
    )
}

const SUPPORTED_SHELLS: [&str; 2] = ["bash", "zsh"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bash_completion_lists_effects_and_shells() {
        let script = get_completion_script(CompletionShell::Bash, &["wipe", "matrix"]);
        assert!(script.contains("wipe matrix"));
        assert!(script.contains("complete -F _ttfx_completion ttfx"));
        assert!(script.contains("complete -F _ttfx_completion terminaltexteffects"));
    }

    #[test]
    fn zsh_completion_lists_effects_and_shells() {
        let script = get_completion_script(CompletionShell::Zsh, &["wipe", "matrix"]);
        assert!(script.contains("effects=(wipe matrix)"));
        assert!(script.contains("compdef _ttfx ttfx"));
    }

    #[test]
    fn cli_parses_terminal_flags_before_effect_subcommand() {
        let cli = Cli::parse_from(["ttfx", "--seed", "42", "wipe", "--speed", "2"]);
        assert_eq!(cli.terminal.seed, Some(42));
        assert_eq!(cli.effect_name(), Some("wipe"));
        assert_eq!(cli.effect_args(), &["--speed".to_string(), "2".to_string()]);
    }
}
