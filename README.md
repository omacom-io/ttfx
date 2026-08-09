# ttfx

Terminal text effects: a single-binary Rust reimplementation of
[terminaltexteffects](https://github.com/ChrisBuilds/terminaltexteffects) (TTE).

Pipe text in, pick an effect, watch it render:

```sh
ls -la | ttfx decrypt
cat banner.txt | ttfx beams
fortune | ttfx --random-effect
git log --oneline -10 | ttfx matrix
```

## Why

TTE is a terrific effects engine, but it's a Python package — a full interpreter
plus install step for a shell toy that wants to live in your prompt pipeline.
ttfx is the same engine as one static binary with no runtime dependencies and
instant startup. Linux only, built for [Omarchy](https://omarchy.org).

## Fidelity

This is a *parity port*, not an approximation. Given the same input, config, and
random decisions, ttfx produces byte-identical frames to the Python reference —
verified mechanically in CI against a pinned TTE checkout (v0.15.0) with a shared
deterministic RNG on both sides. All effects and terminal options are supported
with the same names and defaults; existing `tte` invocations work with the
binary name swapped. See `plan.md` for the full fidelity contract and
`tools/parity/` for the harness.

Randomness itself is not bit-compatible with Python (ttfx uses xoshiro256++;
`--seed` is deterministic within ttfx). Python plugin effects are not supported.

## Usage

```
<producer> | ttfx [terminal options] <effect> [effect options]
ttfx --help                 # all effects and terminal options
ttfx <effect> --help        # options for one effect
ttfx --random-effect        # surprise me (--include-effects / --exclude-effects to filter)
ttfx --print-completion bash|zsh
```

Terminal options (canvas size/anchoring, color handling, frame rate, text wrap,
…) go before the effect name; effect options after it.

## Building

```sh
cargo build --release           # dev build
cargo build --release --target x86_64-unknown-linux-musl   # static release
```

Tests: `cargo test` (engine goldens + state traces), `tools/parity/run_suite.sh`
(frame parity vs the pinned reference — needs python3), `tools/parity/tty_compare.sh`
(full byte-stream parity), `tools/tests/cli_corpus.sh` (CLI contract).

## License

MIT. The vendored reference under `reference/tte/` is upstream TTE (MIT,
© ChrisBuilds) and is used only by the parity test harness — none of it ships
in the binary.
