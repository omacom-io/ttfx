# ttfx — Rust port of TerminalTextEffects

> Name: `ttfx` (checked clean against Arch repos, AUR, and Debian; `ttx` was rejected —
> taken by fonttools).

A pixel-perfect reimplementation of [terminaltexteffects](https://github.com/ChrisBuilds/terminaltexteffects)
(TTE) v0.15.0 (commit `7a91dd9`) in Rust, shipped as a single static binary with no runtime
dependencies.

## 1. Goals and non-goals

**Goals**

- Visual parity: given the same input, config, and random decisions, ttfx produces
  byte-identical frames to Python TTE. "Pixel-perfect" is verified mechanically, not by eyeball
  (see §7).
- CLI compatibility: same effect names, option names, defaults, metavars, and choices as
  `tte`, so existing invocations (`ls | tte decrypt --typing-speed 2`) work with `ttfx`
  substituted.
- Single static binary (musl), no runtime deps, fast startup.
- All 37 effects.
- Linux only, targeted exclusively at Omarchy (Arch). No macOS/Windows/BSD support.

**Non-goals**

- Bit-identical randomness with Python. TTE uses the module-global Mersenne Twister; we use our
  own PRNG. `--seed` remains supported and is deterministic *within ttfx*.
- The Python library API (embedding TTE in Python programs).
- Python plugin effects (`~/.config/terminaltexteffects/effects/*.py`). Out of scope for v1;
  the effect registry is static.
- Wide-character correctness. TTE itself treats one codepoint as one cell (no wcwidth
  anywhere); we reproduce that behavior for parity and document the limitation. Revisit later
  behind a flag.

## 2. Source of truth

Pin the reference: TTE v0.15.0, commit `7a91dd9ca6ee0c4f4b1484efee0ecac1bb84104e`. Vendor a
clone under `reference/tte/` (git submodule or plain checkout, excluded from the binary) so the
parity harness and future upgrades diff against a fixed target.

Scale of the port: ~22k lines of Python — engine ~3.5k, effects ~12.6k, utils ~3.3k. Expect
roughly the same order of magnitude in Rust.

## 3. Crate layout

Single crate, lib + bin:

```
ttfx/
  Cargo.toml
  src/
    main.rs              # arg dispatch, stdin/file input, effect selection, run loop
    cli.rs               # clap root: global/terminal args, subcommand registry, completions
    engine/
      mod.rs
      terminal.rs        # Terminal: config, output, frame pacing, prep/restore
      canvas.rs          # Canvas: dims, anchoring, coord queries
      input.rs           # ANSI-aware input parser -> character grid (the mini terminal emulator)
      character.rs       # EffectCharacter storage (arena), visibility, neighbors/links
      animation.rs       # Scene, Frame, CharacterVisual, sync/eased stepping
      motion.rs          # Waypoint, Segment, Path, Motion
      events.rs          # EventHandler: Event, Action enums, dispatch
      particles.rs       # ParticlePool, ParticleReset
    utils/
      mod.rs
      easing.rs          # 31 named easings + cubic-bezier make_easing + EasingTracker/SequenceEaser
      geometry.rs        # Coord, circle/rect/bezier/line math
      graphics.rs        # Color, ColorPair, Gradient
      hexterm.rs         # xterm-256 <-> RGB table and nearest-match
      ansi.rs            # escape sequence constants + SGR builders (colorterm + ansitools)
      pycompat.rs        # Python-semantics helpers: round-half-even, floor_div, trunc — see §5
      rng.rs             # Rng trait + default PRNG + Python-shaped helpers (randint, choice, shuffle, ...)
      spanning_tree.rs   # PrimsSimple, PrimsWeighted, RecursiveBacktracker, BreadthFirst
    effects/
      mod.rs             # static registry: name -> (build fn, clap command)
      beams.rs ... wipe.rs   # one file per effect, mirroring the Python effect files
  tools/
    parity/              # Python shim + differ (see §7)
  reference/tte/         # pinned upstream checkout
```

**Dependencies** (kept deliberately short):

- `clap` (derive) — CLI. Each effect's config is a struct with `#[arg(...)]` attributes,
  the direct analog of TTE's `ArgSpec` dataclass-field trick.
- `terminal_size` — terminal dimensions (replaces `shutil.get_terminal_size`, same
  (80, 24) fallback on failure).
- `getrandom` — OS entropy for unseeded runs; `signal-hook` (or a minimal self-pipe via
  `libc`) — SIGINT handling. Statically linked crates are not runtime dependencies; the
  single-binary goal doesn't require hand-rolling entropy or signal plumbing.
- No rand crate: `rng.rs` implements xoshiro256++ plus Python-shaped helpers ourselves —
  we need exact control over call semantics for the parity harness anyway (§7).
- Dev-only: whatever the parity tooling needs (none in the binary).

## 4. Core architecture decisions

### 4.1 Arena + IDs instead of the Python object graph

Python TTE is a web of back-references: character ⇄ animation/motion/event_handler, events
holding Scene/Path/Waypoint objects, `links`/`neighbors` between characters, Paths mutated per
activation. In Rust:

- `Vec<EffectCharacter>` arena owned by the Terminal, addressed by `CharacterId(u32)`
  (TTE already has a monotonic `character_id` — reuse it as the index).
- Scenes/Paths live in per-character maps addressed by `SceneId`/`PathId` (interned string or
  small-index newtype). Waypoints by index within their Path.
- `neighbors`/`links` store `CharacterId`s.
- Event targets are IDs, so `EventHandler` tables are plain data:
  `HashMap<(Event, CallerId), Vec<(Action, Target)>>` with insertion-order Vec for actions
  (dispatch order matters).

### 4.2 Ticking without aliasing — synchronous event dispatch

`character.tick()` → motion first, then animation, and event callbacks fired mid-tick can
mutate *other* characters, activate paths/scenes, and add/remove from `active_characters`
(particles do this constantly). Crucially, Python executes actions **immediately at the
emission point** (`_handle_event`, base_character.py:374): segment events fire in the *middle*
of `Path.step` before the coordinate is computed, and `Motion.move` assigns the returned
coordinate *after* callbacks ran — so e.g. a `SET_COORDINATE` action fired from a segment
event is overwritten by the move's own assignment. A deferred queue drained after
`step`/`move`/`tick` would change observable ordering and produce different frames even with
identical RNG draws. Therefore: **no deferred queue.**

Instead, all engine stepping functions are methods on `EngineCtx` (which owns the character
arena, scenes, paths, events, RNG, clock) operating through IDs, never holding long-lived
borrows of character internals. `EngineCtx::handle_event(caller, event)` is called inline at
exactly the source's emission points, recursing depth-first just like Python's call stack.
Where a step function needs local state across an emission point (e.g. `Path::step`'s segment
walk), that state is plain locals (indices, distances) — not borrows — so the reentrant call
is legal. Path/scene runtime state accessed both by the step and by reentrant actions is
re-fetched by ID after each emission point rather than cached across it.

Effect callbacks (`EventHandler.Callback` in Python — closures over iterator state, e.g.
burn's `_emit_smoke`, and particle reclaim closures capturing the pool) cannot be boxed
closures in the arena: they'd borrow the effect that transitively owns the arena. Instead an
event action stores `Callback(CallbackId, Payload)` with an owned payload; the engine
surfaces fired callbacks to the effect via `Effect::dispatch_callback(&mut self, ctx, character_id,
callback_id, payload)`. Because Python callbacks can themselves trigger more engine work,
`dispatch_callback` receives `&mut EngineCtx` and may recurse into engine calls — the effect
struct and the engine context are separate ownership trees, so this borrows cleanly.
`ParticlePool`s live in effect state and are addressed by `PoolId`; reclaim-on-event is a
`Callback` carrying the pool id. No `Rc<RefCell>` anywhere.

### 4.3 One Terminal, deterministic ordering

Python constructs **two** Terminals per run (one owns the tty, one owns the simulation).
Collapse to one `Terminal` (simulation + renderer) plus a thin `TtyWriter` (prep canvas,
frame pacing, restore cursor — RAII `Drop` replaces the `@contextmanager`).

**Ordering is behavior.** Python iterates unordered sets in several behaviorally relevant
places — not just the two engine sites (`active_characters` ticking, render layer-ties) but
also inside effects (middleout, unstable iterate sets directly) and `BreadthFirst`, which
traverses the `links` *set*. And dict insertion order is load-bearing too: swarm chains
`motion.paths.values()`, rings iterates `rings.values()`, and equal-frequency input-color
sorting inherits dict insertion order. Rules:

- **M1 deliverable: a complete inventory of every unordered iteration** (engine + all 37
  effects + spanning trees), each assigned a canonical deterministic order (usually ascending
  `CharacterId` or insertion order). The parity shim patches each Python site to the same
  canonical order (§7).
- Anywhere Python iterates a dict's values/items, Rust uses `Vec` + id→index lookup or an
  insertion-ordered map — plain `HashMap` iteration is only allowed for lookup-only data.
- Render: sort visible characters by `(layer, character_id)`; tick: snapshot
  `active_characters` sorted by `CharacterId` (Python snapshots a tuple).

**`character_id` is not a dense arena index.** Python allocates an id for *every* parsed
character, including ones later overwritten by cursor-movement sequences, popped as trailing
whitespace, or cropped by the canvas — so ids have gaps relative to surviving characters, and
downstream id-ordered iteration depends on the original allocation order. The arena keeps
slot index and `character_id` as separate values; the id counter consumes exactly as the
Python parser does.

### 4.4 Effects as a trait + static registry

```rust
pub trait Effect {
    fn build(&mut self, ctx: &mut EngineCtx);          // Python __init__/build()
    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String>;  // __next__
}
```

`effects/mod.rs` holds the static registry (name → clap `Command` + constructor), replacing
`pkgutil` discovery. `--random-effect`/`--include-effects`/`--exclude-effects`/`--seed` work
as upstream, including the quirk that a randomly selected effect runs with pure default config.

### 4.5 Config system

Terminal config: one clap struct with the 15 upstream options, same defaults
(`--tab-width 4`, `--frame-rate 60`, `--canvas-width -1`, `--anchor-canvas sw`, ...), same
value validation as `argutils` (PositiveInt, ratio ranges, hex-or-xterm ColorArg, etc. —
implemented as clap value parsers in one module). Effect configs: one struct per effect with
`#[arg]` attributes. The shared `final_gradient_*` args can't be a naively `#[flatten]`-ed
struct because each effect overrides their defaults: parse them as `Option<T>` in the shared
struct and merge with the effect's own `Default` after parsing (or generate per-effect arg
declarations with a small macro — decide at M0 with a two-effect spike). Help text copied
from upstream so `--help` is familiar (exact help formatting parity is *not* a goal; option
surface parity is). Usage errors exit 2 (clap's default, matching argparse); runtime errors
exit 1 with upstream's stream choice — file errors print to *stdout*, unsupported-ANSI to
*stderr* (yes, really).

### 4.6 RNG

`rng.rs` defines the engine RNG with Python-shaped methods matching every call TTE makes
(counts from the survey): `randint(a,b)` (61 uses), `choice(&[T])` (54), `shuffle`
(Fisher-Yates, 13), `randrange` (13), `uniform(a,b)` (12), `random()` (12). Backed by
xoshiro256++; seeded from `--seed` or OS entropy (`getrandom` crate). The RNG lives on
`EngineCtx` and is threaded explicitly — no globals — which is also what makes the parity
harness possible (§7). Helper semantics are pinned exactly (e.g. `choice` =
`seq[randbelow(len)]`, `uniform` = `a + (b-a)*random()`, Fisher-Yates in Python's order)
because the Python shim mirrors them.

### 4.7 Clock injection

Two effects read wall/monotonic time directly: matrix (`time.time()` for rain-phase
transitions) and thunderstorm (`time.monotonic()` for the storm budget). Real clocks would
make parity depend on execution speed — with `frame_rate=0` a faster implementation produces
more frames and consumes more RNG draws before a deadline. So `EngineCtx` carries a `Clock`
(`now_wall()`, `now_monotonic()`): the production impl reads real time; the parity impl is
virtual, advancing a fixed `1/frame_rate` per emitted frame. The Python shim monkeypatches
`time.time` and `time.monotonic` with the same virtual clock (§7). This also bounds parity
dump sizes for the time-budgeted effects.

## 5. Fidelity traps (must-reproduce semantics)

These are places where a natural Rust translation silently diverges from Python. All go
through `pycompat.rs` helpers with tests pinned to Python-generated golden values.

1. **`round()` is banker's rounding** (half-to-even). Used for every coordinate quantization,
   `Path.max_steps`, animation frame indices, `adjust_color_brightness`. Rust's `f64::round`
   is half-away-from-zero. → `pycompat::round_half_even(f64) -> i64` used everywhere Python
   calls `round`.
2. **Gradient channel deltas use integer floor division** (`(end - start) // steps`). Python
   `//` floors; Rust `/` on integers truncates toward zero — they differ for negative deltas,
   and gradients are *not* float lerp. → `pycompat::floor_div`, and `Gradient::generate`
   transcribes the integer algorithm exactly (including appending the exact end stop per pair,
   the shared-stop skip, and `loop` appending stop[0]).
3. **Truncation vs rounding is inconsistent upstream and must stay that way**:
   `shift_color_towards` uses `int(x*255)` (truncation); `adjust_color_brightness` uses
   `round(x*255)` (half-to-even). Reproduce each as-is.
4. **`find_length_of_bezier_curve` omits the final t=0.9→1.0 span** (10-sample loop bug).
   Path lengths are systematically short; `max_steps` depends on it. Reproduce the bug.
5. **Row deltas are doubled** in path distances (`double_row_diff=True`), circle x-offsets are
   doubled, diagonal/radial gradient math doubles rows — the cell-aspect convention. Copy each
   call site's choice exactly (`geometry::line_length(a, b, double_row: bool)` defaults off,
   like upstream).
6. **Unclamped eased `t`**: `Path.step` deliberately allows `t > 1`/`t < 0` for overshooting
   easings (back/elastic) including traveling *past* the final waypoint via the for-else
   overshoot re-add; `_step_eased_scene` clamps instead. Copy both behaviors.
7. **`hex_to_xterm` metric** is minimum *mean absolute channel difference* via linear scan over
   the 256-entry table (first minimum wins) — not Euclidean, not perceptual. Port the table and
   scan order verbatim.
8. **Looping-scene semantics**: `active_scene_is_complete()` returns true for looping scenes
   (so loop-only characters get pruned from `active_characters`), and `SCENE_COMPLETE` fires
   *every tick* for looping scenes. Effects depend on both quirks.
9. **`activate_scene` does not reset playback** (resume semantics); `reset_scene` restores
   played + remaining frames into the frame queue in original order, zeroes each frame's
   `ticks_elapsed` and the easing step, and clears `played_frames`. Verify with
   partial-playback tests.
10. **Path re-activation mutates the path**: a synthetic origin segment from the current
    coordinate is built, the *previous* origin segment's distance is subtracted and the new
    one's added (rebase, not cumulative accumulation), segments[0] replaced or inserted,
    `current_step`/`hold_time_remaining` reset, `max_steps` recomputed. Copy exactly.
11. **Segment events key off the end Waypoint**, fire once per activation, and do not re-fire
    on backwards easing motion. Copy the flag mechanics.
12. **Synced-scene formulas** (`STEP`/`DISTANCE` with their `max(...,1)` guards and
    `round()` indexing) transcribed exactly; missing active path → jump to last frame and
    force-complete.
13. **Input parsing**: the mini ANSI emulator (CSI-only, specific SGR subset, bold bumping a
    pending standard fg by +8, `\t` → N space characters, cursor-movement sequences, the
    unsupported-sequence error), trailing space/line trimming, plain uncolored spaces demoted
    to fill characters, `"No Input."` fallback, empty-stdin-when-tty → `""`. Transcribe
    directly with the same error taxonomy. Stateful quirks that must survive transcription:
    *unsupported SGR parameter values are silently ignored* (the SGR loop has no error
    fallback — only malformed/unsupported *sequences* raise), and `_input_colors_frequency`
    increments at character-creation time, so colors of cells later overwritten by cursor
    movement still count (affects `get_input_colors` and RNG ordering). Differential-test the
    ugly corpus: cursor overwrites, trailing colored cells, ignored SGR params, private
    modes, malformed CSI.
14. **Canvas math**: center formulas (`top//2` + odd adjustment), anchor offset computation,
    visible-bounds clamping, `outside_scope` random coords exactly one cell beyond an edge,
    `-1`/`0` canvas sizing semantics, `--ignore-terminal-dimensions` overwriting terminal dims.
15. **`get_characters` sort algorithms** including the destructive alternate-pop interleave for
    outside/middle sorts (order matters, O(n²) is fine), and grouped variants' exact grouping
    (diagonal bands by `row+column` / `column-row`, Manhattan-distance bucketing from
    `text_center`).
16. **CharacterVisual SGR order**: bold, italic, underline, blink, reverse, hidden, strike,
    fg, bg, symbol, `\x1b[0m`; `dim` stored but never emitted; bare symbol when unformatted.
    Frame string = rows joined `"\n"`, top row first. Row dimensions are **`visible_top` ×
    `visible_right`** — absolute terminal-space extents after canvas anchoring and clipping —
    *not* canvas width/height; non-southwest anchors produce leading blank rows/columns inside
    every frame. Specify the renderer in `visible_*`/offset terms and golden-test all nine
    anchors under both clipped and unclipped terminal sizes.
17. **`existing_color_handling`** three modes including `"always"` overriding every
    `add_frame`/`set_appearance` color and applying at parse time; `preexisting_colors_present`
    scan. `Color` equality is on the *original argument* (`Color(255) != Color("ffffff")`) —
    keep that for `input_colors_frequency` and any keying.
18. **`find_normalized_distance_from_center` stays in [0,1]** for accepted coordinates (it
    rejects out-of-rectangle coords with a `ValueError`, and the doubled-row diagonal bounds
    the numerator); `Gradient::get_color_at_fraction` validates `0 <= f <= 1` and errors
    otherwise. Reproduce both the rejection and the validation as errors (don't clamp), and
    pin with exhaustive rectangle-lattice tests.
19. **Frame pacing**: `1/frame_rate` delay, `monotonic` check, sleep remainder, timestamp
    taken *after* sleep (drift accumulates), `frame_rate == 0` disables. Same ANSI prep/restore
    dance: hide cursor, scroll-make-room (or `reuse_canvas` reposition), DEC save (`\x1b7`),
    per-frame DEC restore/save + cursor-up, restore cursor + newline on exit (respecting
    `--no-eol`/`--no-restore-cursor`), teardown running even on error/ctrl-C (RAII +
    SIGINT handler; Python exits 1 silently on KeyboardInterrupt).
20. **Float expression order**: transcribe arithmetic expressions in the same order/grouping as
    Python — don't "simplify" math during port (easing functions, HSL round-trips, bezier De
    Casteljau with float intermediates rounded only at the end). Caveat: identical expression
    order guarantees identical results only for IEEE basic ops (+,-,*,/,sqrt). Transcendentals
    (`sin`, `cos`, `pow`, `exp`) and `hypot` go through libm, and musl-Rust vs glibc-CPython
    can differ by ulps. Mitigation: parity CI runs on one pinned platform/toolchain pair;
    comparisons happen on *quantized* outputs (rounded coordinates, hex colors), which absorb
    ulp noise except exactly at rounding boundaries; fine-grained easing/geometry goldens
    flag any boundary case early, and such cases get boundary-tolerant assertions rather
    than pretending bit-identity.

**Deliberate divergences** (accepted, documented):

- PRNG (xoshiro vs MT19937) and therefore any `--seed` output.
- Set-iteration orderings replaced with `character_id` orderings (§4.3).
- The `lru_cache` layers (geometry, `shift_color_towards`, `make_easing`) are dropped —
  they're pure-function caches, so behavior is identical; Rust recomputation is cheaper than
  Python cache hits. If profiling disagrees, memoize `make_easing`'s Newton–Raphson solve only.
  **Addendum (found during the swarm port):** the value-transparency claim fails when an
  effect MUTATES a cached return value — swarm `random.shuffle`s the list returned by the
  cached `find_coords_on_circle`, so later same-argument calls observe the shuffled entry.
  Such effects reproduce the cache at effect level (a persistent map whose entries carry the
  mutation); audit any effect that writes to a geometry function's return value.
- The `AldousBroder` spanning-tree generator is not ported: it exists upstream but no shipped
  effect uses it (library-API surface, which is a non-goal).
- No Python plugin loading.
- Error-message *text* may differ; error *conditions* and exit codes match.

## 6. Porting strategy

Transcription, not reimagination: each Python file maps to one Rust file; functions keep their
names and internal structure; comments reference upstream line numbers for anything subtle.
The two survey documents (engine architecture + effect catalog) serve as the map; the pinned
checkout is the letter.

**Phase order** (each phase lands with its parity tests green):

- **M0 — skeleton + input pipeline.** Cargo scaffold, CLI root with terminal config, stdin/file
  input, the input parser (§5.13), Canvas + anchoring, fill characters, neighbors, renderer,
  `TtyWriter`. Exit criterion: piping text through a no-op "effect" reproduces Python's
  preprocessed first frame byte-for-byte across the anchor/canvas/wrap/tab/existing-color
  option matrix.
- **M1 — engine core.** easing (31 + `make_easing`), geometry, graphics, hexterm, animation,
  motion, events, particles, spanning trees, `pycompat`, `rng`, clock. Exit criteria:
  (a) golden-value tests for every pure function (inputs + expected outputs generated by a
  Python script run against the pinned checkout, committed as fixtures); (b) the upstream
  engine test suite (`tests/engine_tests/`) ported — it already covers multi-segment event
  ordering, particle reclaim/reset, and scene lifecycle; (c) scripted state-trace tests for
  the delicate machinery: nested/reentrant events, scene reactivation-resume, looping scenes,
  path holds and loop-rebase, pool exhaustion/reuse; (d) the complete unordered-iteration
  inventory (§4.3) written down with its canonical orders.
- **M2 — parity harness** (§7). Exit criterion: harness runs end-to-end on a hand-written
  trivial effect in both implementations with byte-identical frame streams.
- **M3 — effects, wave 1 (motion + scenes basics):** randomsequence, wipe, expand, slice,
  scattered, pour, slide, middleout, spray, rain, bouncyballs, errorcorrect. (~12 effects,
  each small.)
- **M4 — effects, wave 2 (gradients, sync, sequence easers, no-motion effects):** colorshift,
  highlight, sweep, waves, decrypt, print, overflow, unstable, crumble, blackhole, swarm,
  spotlights, fireworks, bubbles, beams, rings, orbittingvolley, binarypath.
- **M5 — effects, wave 3 (heavy machinery):** burn + smoke (spanning trees), laseretch
  (backtracker + particles + bezier), vhstape, synthgrid, matrix, thunderstorm. The top-5
  hairiest land last, when the engine is fully proven.
- **M6 — CLI completion + polish.** `--print-completion bash|zsh` (static generation from the
  clap model, mirroring upstream's completion behavior), `--version`, `--random-effect`
  filtering, error paths/exit codes, README.
- **M7 — release engineering.** `x86_64-unknown-linux-musl` static build (add
  `aarch64-unknown-linux-musl` only if Omarchy grows an ARM target), LTO + strip, CI running
  the full parity suite against the pinned checkout, benchmark vs Python (startup + big-input
  throughput), release artifacts + Arch packaging (PKGBUILD) for Omarchy.

Each effect PR ships: the ported effect, its clap config struct, and its parity-suite entry.

## 7. Parity verification (what makes "pixel-perfect" checkable)

The problem: effects are random, so naive frame-diffing fails. The solution: make randomness a
*shared injected dependency* during tests.

**Deterministic RNG shim.** Implement one trivially-portable PRNG (xoshiro256++, fixed seed)
identically in Rust (`rng.rs`, it's the real RNG) and in a small Python module
(`tools/parity/shim.py`). The shim monkeypatches `random.randint/choice/shuffle/randrange/
uniform/random` with the shared implementation before importing TTE. Both sides now draw
identical random sequences — *provided the port makes RNG calls in the same order as Python*,
which is exactly what faithful transcription gives us, and precisely what the harness verifies.

**Determinism patches.** The shim also patches every site in the §4.3 inventory —
`BaseEffectIterator.update`, render layer-tie ordering, effect-level set iterations
(middleout, unstable, ...), `BreadthFirst`'s `links`-set traversal — to the same canonical
orders as the Rust port. These patches pin *which* of Python's arbitrary orderings we compare
against.

**Clock patch.** The shim replaces `time.time`/`time.monotonic` with the virtual clock
(§4.7), advancing `1/frame_rate` per frame, so matrix and thunderstorm are deterministic and
their dumps bounded.

**Shim audit.** Because the shim modifies the reference, we separately guard against "proving
parity with a modified TTE": every deterministic (RNG-free, clock-free) effect and the whole
M0 preprocessing matrix are *also* byte-compared against a completely unmodified pinned
CPython run; and the shim's patches are limited by construction to ordering/RNG/clock
substitution (a diff of shimmed-vs-unmodified TTE source is reviewed, small, and committed
alongside the harness).

**Frame capture.** Python side: shim iterates the effect with `frame_rate=0` and fixed canvas
(`--canvas-width/height` explicit + `--ignore-terminal-dimensions`), writing each frame string
to a length-prefixed dump. Rust side: hidden `--parity-dump <seed>` flag does the same. A
differ compares streams and reports first divergent frame/row/column with a decoded escape
view.

**Test matrix.** Per effect: 2–3 input texts (ASCII multiline, colored-ANSI input, ragged
short input) × default config × 1–2 non-default configs exercising that effect's options.
Plus the option-matrix suite for M0 preprocessing. All checked into CI; the suite regenerates
Python dumps from `reference/tte/` on demand (dumps themselves can be cached by content hash).
Parity CI runs on one pinned platform/toolchain pair (Arch x86_64, pinned CPython + rustc) —
cross-platform builds are release targets, not parity targets (§5.20).

**PTY byte-stream tests.** Frame dumps bypass the tty layer, so a separate small suite runs
both implementations under a pseudo-terminal and byte-compares the *full* output stream:
canvas prep (blank-row scroll, DEC save), per-frame restore/save/cursor-up preamble, and
teardown (cursor show, EOL) — including the `--reuse-canvas`/`--no-eol`/`--no-restore-cursor`
variants and the SIGINT path.

**Fallback tier.** If an effect turns out to interleave RNG draws through a code path whose
order can't reasonably be matched (none identified yet, but e.g. hash-order-dependent draw
sequences inside an effect), it drops to tier-2 verification: structural frame comparison
(non-random subsets byte-compared; random placements checked for distribution/bounds) plus
manual visual sign-off against side-by-side recordings. Goal: zero tier-2 effects.

**Unit goldens.** Independent of frame parity: fixtures generated from Python for easing curves
(sampled at 1e-3 steps), all gradient constructions in the effect defaults, hexterm's full
256-entry nearest-match on a color sweep, geometry functions on a coordinate lattice, and the
input parser on an ANSI corpus.

## 8. Runtime behavior details

- **Input**: stdin (empty when tty), `--input-file`; empty/whitespace input → `NO INPUT.` on
  stdout, exit 1. Decoding is **strict UTF-8** for both file and stdin — upstream file reads
  are strict (decode failure → error message, exit 1) and stdin goes through Python's text
  stream, strict UTF-8 in the environments we target. No lossy decoding; test the failure
  path.
- **Signals**: SIGINT is recorded via flag/self-pipe and *returns control to the main loop*,
  which unwinds normally so the RAII teardown runs (`Drop` alone doesn't fire on a signal);
  exit 1, no message (matches KeyboardInterrupt handling).
- **Exit codes**: 0 success; 1 runtime errors — no-input, missing effect, file errors (message
  on *stdout*, matching upstream), unsupported ANSI sequence (message on *stderr*); 2 usage
  errors from argument parsing (argparse/clap convention). A CLI corpus test covers exit
  codes, stream routing, root-option-before-subcommand placement, `nargs`-style multi-value
  options, negative-looking values, include/exclude filtering, and decode failures.
- **Performance target**: not a goal beyond "never the bottleneck" — full-canvas repaint at
  60fps on a 400×100 canvas is trivial in Rust; the O(n²) upstream algorithms (outside-in
  sort, grouped scans) are kept for fidelity and are fine at terminal scale. Per-cell
  `formatted_symbol` strings are precomputed on visual change exactly as upstream caches them
  in `CharacterVisual`. Frame strings are freshly allocated `String`s at first (simple,
  matches the `Option<String>` iterator shape); buffer recycling only if profiling ever says
  so.

## 9. Risks

| Risk | Mitigation |
|---|---|
| RNG call-order divergence in some effect makes frame parity unachievable there | Faithful transcription makes order match by construction; harness catches drift per-effect at port time, not at the end; tier-2 fallback exists |
| Hidden reliance on Python set/hash/dict order beyond the M1 inventory | Parity harness surfaces it as a reproducible first-divergence; the site joins the inventory, the shim gains a patch, Rust matches it |
| Float divergence (expression order or libm ulps) | §5.20: transcribe expression order, pin the parity platform, compare quantized outputs, fine-grained goldens catch boundary cases early |
| Shim-validated parity diverges from *unmodified* TTE | Shim audit (§7): deterministic effects + preprocessing byte-compared against unpatched CPython; shim diff kept minimal and reviewed |
| clap can't express an argparse corner (e.g. `nargs` tuple actions, dual-type parsers like laseretch's etch-pattern) | Custom value parsers; worst case a manual `TypedValueParser` per odd option — all identified odd options are enumerated in the survey |
| Upstream moves on and the port targets a stale version | Pinned reference is explicit; upgrades are a diff of `reference/tte/` + re-run of parity suite |
| Scope creep into "improving" TTE | §5 divergence list is the only allowed list; everything else is transcription |

## 10. Open questions (decide before M0)

1. Also ship a `tte` alias/symlink in packaging?
2. Vendor upstream as git submodule vs plain checked-in snapshot (parity CI needs it; snapshot
   is simpler, submodule is cleaner). → Decided: plain snapshot, simplest for CI.
