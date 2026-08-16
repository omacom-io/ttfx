//! ttfx: a Rust port of TerminalTextEffects (TTE) v0.15.0.
//!
//! This crate exposes the engine, effects, CLI, and utility modules used by
//! the `ttfx` binary (see `src/main.rs`). Visual/CLI parity target: TTE
//! v0.15.0, commit `7a91dd9ca6ee0c4f4b1484efee0ecac1bb84104e`.

pub mod cli;
pub mod engine;

// `effects` is implemented as a directory module (`src/effects/mod.rs` plus
// one file per effect). An explicit `#[path]` pins module resolution to that
// file so the presence of a stray `src/effects.rs` on disk does not trigger
// rustc's default-vs-mod.rs ambiguity check (E0761); only the path named
// here is compiled as the `effects` module.
#[path = "effects/mod.rs"]
pub mod effects;

pub mod utils;
