//! Root CLI surface: global/terminal arguments, the effect subcommand
//! registry, and shell completion generation.
//!
//! Mirrors `terminaltexteffects/__main__.py::build_parser` (argparse root +
//! effect discovery) and `terminaltexteffects/utils/shell_completion.py`
//! (bash/zsh completion emission). The effect subcommands themselves are
//! *not* known here: `effects/mod.rs` builds the static registry of
//! `(name, fn() -> clap::Command)` pairs and hands it to [`build_root_command`],
//! replacing Python's `pkgutil`-based module discovery + XDG plugin loading.
//!
//! Usage errors exit 2 (clap's default, matching argparse's `SystemExit(2)`).
//! Runtime errors are handled by the caller (see plan §4.5): file errors go
//! to stdout, unsupported-ANSI errors to stderr, matching upstream's choice.

use std::path::PathBuf;

use clap::{Args, Command, FromArgMatches, ValueEnum};

/// Canvas/text anchor points, matching TTE's `Anchor` `Literal` values.
///
/// Upstream: `terminaltexteffects/engine/canvas.py` anchor handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Anchor {
    #[value(name = "n")]
    North,
    #[value(name = "ne")]
    NorthEast,
    #[value(name = "e")]
    East,
    #[value(name = "se")]
    SouthEast,
    #[value(name = "s")]
    South,
    #[value(name = "sw")]
    SouthWest,
    #[value(name = "w")]
    West,
    #[value(name = "nw")]
    NorthWest,
    #[value(name = "c")]
    Center,
}

/// `existing_color_handling`: how ANSI color already present in the input
/// interacts with effect-applied color. Matches the `Literal["always",
/// "dynamic", "ignore"]` in `terminaltexteffects/engine/terminal.py`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ExistingColorHandling {
    Always,
    Dynamic,
    Ignore,
}

impl Default for ExistingColorHandling {
    fn default() -> Self {
        ExistingColorHandling::Ignore
    }
}

/// Terminal-level configuration flags, flattened into every invocation.
///
/// Upstream: `TerminalConfig` fields in `terminaltexteffects/engine/terminal.py`.
#[derive(Args, Debug, Clone)]
pub struct TerminalArgs {
    /// Ignore the detected terminal dimensions and use the input dimensions.
    #[arg(long = "ignore-terminal-dimensions", action = clap::ArgAction::SetTrue)]
    pub ignore_terminal_dimensions: bool,

    /// Override the detected canvas width. Falls back to terminal width.
    #[arg(long = "canvas-width", value_name = "N")]
    pub canvas_width: Option<i32>,

    /// Override the detected canvas height. Falls back to terminal height.
    #[arg(long = "canvas-height", value_name = "N")]
    pub canvas_height: Option<i32>,

    /// Anchor point for the canvas within the terminal.
    #[arg(long = "anchor-canvas", value_enum, default_value_t = Anchor::SouthWest)]
    pub anchor_canvas: Anchor,

    /// Anchor point for the input text within the canvas.
    #[arg(long = "anchor-text", value_enum, default_value_t = Anchor::Center)]
    pub anchor_text: Anchor,

    /// How to treat ANSI color sequences already present in the input.
    #[arg(long = "existing-color-handling", value_enum, default_value_t = ExistingColorHandling::Ignore)]
    pub existing_color_handling: ExistingColorHandling,

    /// Disable color output entirely.
    #[arg(long = "no-color", action = clap::ArgAction::SetTrue)]
    pub no_color: bool,

    /// Force xterm-256 color output instead of truecolor.
    #[arg(long = "xterm-colors", action = clap::ArgAction::SetTrue)]
    pub xterm_colors: bool,

    /// Wrap text that exceeds the canvas width instead of clipping it.
    #[arg(long = "wrap-text", action = clap::ArgAction::SetTrue)]
    pub wrap_text: bool,

    /// Number of spaces a tab character expands to.
    #[arg(long = "tab-width", default_value_t = 4u8)]
    pub tab_width: u8,

    /// Target frame rate, in frames per second.
    #[arg(long = "frame-rate", default_value_t = 100.0f64)]
    pub frame_rate: f64,
}

/// Global arguments accepted before/around the effect subcommand.
///
/// Upstream: the root-level `ArgumentParser` arguments in
/// `terminaltexteffects/__main__.py::build_parser`.
#[derive(Args, Debug, Clone)]
pub struct GlobalArgs {
    /// Read input from FILE instead of stdin.
    #[arg(long = "input-file", value_name = "FILE")]
    pub input_file: Option<PathBuf>,

    /// Seed the RNG for reproducible output. Omit for OS entropy.
    #[arg(long = "seed", value_name = "N")]
    pub seed: Option<u64>,

    /// Select a random effect instead of the named subcommand. Runs with
    /// pure default configuration, matching upstream's quirk.
    #[arg(long = "random-effect", action = clap::ArgAction::SetTrue)]
    pub random_effect: bool,

    /// Restrict `--random-effect` to this list of effect names.
    #[arg(long = "include-effects", num_args = 1.., value_name = "EFFECT")]
    pub include_effects: Vec<String>,

    /// Exclude these effect names from `--random-effect` selection.
    #[arg(long = "exclude-effects", num_args = 1.., value_name = "EFFECT")]
    pub exclude_effects: Vec<String>,

    #[command(flatten)]
    pub terminal: TerminalArgs,
}

/// Static registry of effect subcommands: name paired with a constructor
/// for its `clap::Command`. Populated by `effects/mod.rs`; this module only
/// knows how to merge it into the root command.
pub type EffectRegistry = Vec<(&'static str, fn() -> Command)>;

/// Shells supported by the completion generator.
///
/// Upstream: `SUPPORTED_SHELLS` in `terminaltexteffects/utils/shell_completion.py`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
}

fn build_completions_command() -> Command {
    Command::new("completions")
        .about("Generate a shell completion script.")
        .arg(
            clap::Arg::new("shell")
                .value_parser(clap::builder::EnumValueParser::<Shell>::new())
                .required(true),
        )
}

/// Build the full root command: global/terminal args flattened in, plus the
/// `completions` subcommand, plus every registered effect subcommand.
pub fn build_root_command(registry: &EffectRegistry) -> Command {
    let mut root = Command::new("ttfx")
        .about("Apply visual effects to terminal text output.")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(false)
        .arg_required_else_help(false);

    root = GlobalArgs::augment_args(root);
    root = root.subcommand(build_completions_command());

    for (_name, build) in registry {
        root = root.subcommand(build());
    }

    root
}

/// Result of parsing the full command line: global config plus, if an
/// effect subcommand was invoked, its name and matched arguments.
pub struct ParsedCli {
    pub global: GlobalArgs,
    pub effect: Option<(String, clap::ArgMatches)>,
}

/// Parse `std::env::args_os()` against the merged root command.
///
/// Usage errors exit the process with code 2 via clap's default error
/// handling in `Command::get_matches`, matching argparse's behavior.
pub fn parse(registry: &EffectRegistry) -> ParsedCli {
    let cmd = build_root_command(registry);
    let matches = cmd.get_matches();

    let global =
        GlobalArgs::from_arg_matches(&matches).expect("global args validated by clap parsing");
    let effect = matches
        .subcommand()
        .filter(|(name, _)| *name != "completions")
        .map(|(name, sub)| (name.to_string(), sub.clone()));

    ParsedCli { global, effect }
}

/// A single option's completion metadata, gathered from a `clap::Arg`.
///
/// Upstream: `CompletionOption` dataclass in
/// `terminaltexteffects/utils/shell_completion.py`.
#[derive(Debug, Clone)]
struct CompletionOption {
    option_strings: Vec<String>,
    choices: Vec<String>,
    takes_value: bool,
}

fn takes_value(arg: &clap::Arg) -> bool {
    !matches!(
        arg.get_action(),
        clap::ArgAction::SetTrue
            | clap::ArgAction::SetFalse
            | clap::ArgAction::Count
            | clap::ArgAction::Help
            | clap::ArgAction::Version
    )
}

fn collect_options(cmd: &Command) -> Vec<CompletionOption> {
    cmd.get_arguments()
        .filter(|a| a.get_id() != "help" && a.get_id() != "version")
        .map(|arg| {
            let mut option_strings = Vec::new();
            if let Some(long) = arg.get_long() {
                option_strings.push(format!("--{long}"));
            }
            if let Some(short) = arg.get_short() {
                option_strings.push(format!("-{short}"));
            }
            let choices = arg
                .get_possible_values()
                .iter()
                .map(|v| v.get_name().to_string())
                .collect();
            CompletionOption {
                option_strings,
                choices,
                takes_value: takes_value(arg),
            }
        })
        .filter(|opt| !opt.option_strings.is_empty())
        .collect()
}

/// Escape a list of words for safe inclusion inside a double-quoted shell
/// string, mirroring `_escape_for_shell` upstream.
fn escape_for_shell<I, S>(words: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    words
        .into_iter()
        .map(|w| w.as_ref().replace('\\', "\\\\").replace('"', "\\\"") )
        .collect::<Vec<_>>()
        .join(" ")
}

fn generate_bash_completion(root: &Command, registry: &EffectRegistry) -> String {
    let mut effect_names: Vec<&str> = registry.iter().map(|(name, _)| *name).collect();
    effect_names.push("completions");

    let mut script = String::new();
    script.push_str("# ttfx bash completion\n");
    script.push_str("_ttfx_completions() {\n");
    script.push_str("    local cur prev words cword\n");
    script.push_str("    _init_completion || return\n");
    script.push_str(&format!(
        "    local effects=\"{}\"\n",
        escape_for_shell(&effect_names)
    ));

    for (name, build) in registry {
        let sub = build();
        let opts = collect_options(&sub);
        let flat: Vec<String> = opts
            .iter()
            .flat_map(|o| o.option_strings.clone())
            .collect();
        script.push_str(&format!(
            "    if [[ \"${{words[1]}}\" == \"{name}\" ]]; then\n        COMPREPLY=($(compgen -W \"{}\" -- \"$cur\"))\n        return\n    fi\n",
            escape_for_shell(&flat)
        ));
    }

    let root_opts = collect_options(root);
    let root_flat: Vec<String> = root_opts
        .iter()
        .flat_map(|o| o.option_strings.clone())
        .collect();
    script.push_str(&format!(
        "    COMPREPLY=($(compgen -W \"{} $effects\" -- \"$cur\"))\n",
        escape_for_shell(&root_flat)
    ));
    script.push_str("}\ncomplete -F _ttfx_completions ttfx\n");
    script
}

fn generate_zsh_completion(root: &Command, registry: &EffectRegistry) -> String {
    let mut effect_names: Vec<&str> = registry.iter().map(|(name, _)| *name).collect();
    effect_names.push("completions");

    let mut script = String::new();
    script.push_str("#compdef ttfx\n");
    script.push_str("_ttfx() {\n    local -a effects\n");
    script.push_str(&format!(
        "    effects=({})\n",
        effect_names.join(" ")
    ));

    let root_opts = collect_options(root);
    let root_flat: Vec<String> = root_opts
        .iter()
        .flat_map(|o| o.option_strings.clone())
        .collect();
    script.push_str(&format!(
        "    _arguments -C \\\n        '1:command:({} $effects)' \\\n        '*::arg:->args'\n",
        root_flat.join(" ")
    ));

    for (name, build) in registry {
        let sub = build();
        let opts = collect_options(&sub);
        let flat: Vec<String> = opts
            .iter()
            .flat_map(|o| o.option_strings.clone())
            .collect();
        script.push_str(&format!(
            "    if [[ \"${{words[1]}}\" == \"{name}\" ]]; then\n        _values '{name} options' {}\n    fi\n",
            flat.iter()
                .map(|s| format!("'{s}'"))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }

    script.push_str("}\n_ttfx \"$@\"\n");
    script
}

/// Generate a completion script for the given shell, covering the root
/// command's global options and every registered effect's own options.
pub fn generate_completions(shell: Shell, registry: &EffectRegistry) -> String {
    let root = build_root_command(registry);
    match shell {
        Shell::Bash => generate_bash_completion(&root, registry),
        Shell::Zsh => generate_zsh_completion(&root, registry),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_registry() -> EffectRegistry {
        Vec::new()
    }

    #[test]
    fn root_command_builds_and_has_completions_subcommand() {
        let registry = empty_registry();
        let cmd = build_root_command(&registry);
        assert!(cmd.find_subcommand("completions").is_some());
    }

    #[test]
    fn global_args_parse_defaults() {
        let registry = empty_registry();
        let cmd = build_root_command(&registry);
        let matches = cmd.try_get_matches_from(["ttfx"]).unwrap();
        let global = GlobalArgs::from_arg_matches(&matches).unwrap();
        assert!(!global.random_effect);
        assert_eq!(global.terminal.tab_width, 4);
        assert_eq!(global.terminal.existing_color_handling, ExistingColorHandling::Ignore);
    }

    #[test]
    fn completion_scripts_are_nonempty() {
        let registry = empty_registry();
        assert!(!generate_completions(Shell::Bash, &registry).is_empty());
        assert!(!generate_completions(Shell::Zsh, &registry).is_empty());
    }
}
