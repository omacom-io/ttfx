# ttfx

Terminal text effects as a single static binary. Pipe text in, pick an effect:

```sh
ls -la | ttfx decrypt
cat banner.txt | ttfx beams
fortune | ttfx --random-effect
git log --oneline -10 | ttfx matrix
```

## Credit where it's due

**This is a port of [TerminalTextEffects](https://github.com/ChrisBuilds/terminaltexteffects)
(TTE) by [ChrisBuilds](https://github.com/ChrisBuilds).** Every effect, the animation engine,
and the command-line interface are their design — this project translates that work to Rust
and adds nothing of its own to the art. If you like what you see here, star the original.

TTE is MIT licensed and so is this port; the original copyright is preserved in
[LICENSE](LICENSE) and [NOTICE](NOTICE). Please file *effect* ideas upstream, where they belong.

## Why a port

TTE is a Python package. That's the right call for a library, but for a shell toy that lives in
your prompt pipeline it means an interpreter, an install step, and ~90 ms of import before the
first frame. ttfx is one dependency-free binary that starts in ~1 ms.

That difference is the whole reason this exists. On a fullscreen canvas the heavier effects
can't hold a high frame rate under Python:

| At 200×50 cells | ttfx | Python TTE |
|---|---|---|
| beams | 564 fps | 71 fps |
| slide | 5,113 fps | 264 fps |
| waves | 4,118 fps | 491 fps |
| startup | 1.2 ms | 107 ms |

Across all 37 effects the median speedup is **9.6×** (range 4.5×–21.6×).

## Fidelity

This is a *parity port*, not a reimplementation-in-spirit. Given the same input, config, and
random draws, ttfx produces **byte-identical frames** to the Python original — verified
mechanically in CI against a pinned upstream checkout (v0.15.0), not by eyeballing.

| Suite | Checks | What it proves |
|---|---|---|
| `tools/parity/run_suite.sh` | 354 | every effect's frame stream, byte for byte, across configs and seeds |
| `tools/parity/tty_compare.sh` | 41 | the full terminal byte stream — canvas prep, cursor moves, teardown |
| `tools/tests/cli_corpus.sh` | 19 | exit codes and stdout/stderr routing |
| `cargo test` | goldens + traces | easing/geometry/gradient values and engine state machines |

Making that possible meant reproducing upstream's quirks deliberately, not "fixing" them:
Python's banker's rounding, gradients built from integer floor division rather than float
interpolation, a bezier arc-length approximation that drops its final segment, and looping
scenes that report themselves complete on every tick. They're catalogued in
[`plan.md`](plan.md); the places where Python's unordered iteration had to be pinned down are
in [`docs/ordering-inventory.md`](docs/ordering-inventory.md).

**Two deliberate differences.** Random number generation is not bit-compatible with CPython —
ttfx uses xoshiro256++, so `--seed` is reproducible within ttfx but won't match Python's
Mersenne Twister. (The parity harness swaps a shared PRNG into both sides, which is what makes
frame comparison possible at all.) And Python plugin effects aren't supported, since there's
no interpreter to load them.

## Usage

```
<producer> | ttfx [terminal options] <effect> [effect options]

ttfx --help                 # all 37 effects and the terminal options
ttfx <effect> --help        # options for one effect
ttfx --random-effect        # surprise me (--include-effects / --exclude-effects to filter)
ttfx --print-completion bash|zsh
```

Terminal options (canvas size and anchoring, color handling, frame rate, text wrapping) go
before the effect name; effect options after it. Option names and defaults match `tte`, so
existing invocations work with the binary name swapped.

## Building

```sh
cargo build --release
cargo build --release --target x86_64-unknown-linux-musl   # static, ~3.3 MB
```

Running the parity suites needs python3 and a copy of upstream:

```sh
./tools/parity/fetch_reference.sh   # clones TTE at the pinned commit
./tools/parity/run_suite.sh
```

Upstream is not vendored here — the harness fetches it, because it's their code.

## Scope

Linux and macOS. Built for [Omarchy](https://omarchy.org) originally; nothing targets a
specific libc, and CI runs the tests and CLI corpus on both platforms. The byte-exact
parity suites stay pinned to Linux/glibc — Apple's libm rounds a few transcendentals a
last-ulp differently, which quantization hides in real frames but a bit-exact comparison
would surface.

## License

MIT — see [LICENSE](LICENSE), which carries both this project's copyright and the original
TerminalTextEffects copyright, and [NOTICE](NOTICE) for the attribution in full.
