"""Fair ttfx vs upstream-TTE benchmark.

Both sides run their real user-facing command (no parity shim — the shim's
pure-Python RNG would unfairly slow CPython, whose own random module is C).
Frame pacing is disabled on both sides so this measures render throughput,
not sleep().

matrix and thunderstorm are reported separately: they gate on wall-clock time,
so a faster implementation renders MORE frames in the same seconds rather than
finishing sooner — a speed ratio would be meaningless.

Usage: bench_full.py [repeats]
"""

from __future__ import annotations

import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RUST = ROOT / "target/release/ttfx"
REF = ROOT / "reference/tte"
REPEATS = int(sys.argv[1]) if len(sys.argv) > 1 else 3
CLOCK_BOUND = {"matrix", "thunderstorm"}

ENV = {**os.environ, "COLUMNS": "100", "LINES": "30", "PYTHONPATH": str(REF)}


def effects() -> list[str]:
    out = subprocess.run([str(RUST), "--help"], capture_output=True, text=True).stdout
    names, grab = [], False
    for line in out.splitlines():
        if line.startswith("Commands:"):
            grab = True
            continue
        if line.startswith("Options:"):
            break
        if grab and line.startswith("  ") and line.strip():
            n = line.split()[0]
            if n != "help":
                names.append(n)
    return names


def best_of(cmd: list[str], data: bytes) -> float:
    """Best wall-clock of REPEATS runs, in ms (best = least noise)."""
    times = []
    for _ in range(REPEATS):
        t = time.monotonic()
        subprocess.run(cmd, input=data, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=ENV)
        times.append((time.monotonic() - t) * 1000)
    return min(times)


def frames_of(effect: str, data: bytes) -> int:
    r = subprocess.run(
        [str(RUST), "--seed", "1", "--frame-rate", "0", "--virtual-clock", "--parity-dump", effect],
        input=data, capture_output=True, env=ENV,
    )
    return int(r.stderr.decode().strip().rsplit("=", 1)[-1] or 0)


def main() -> int:
    text = "\n".join(f"benchmark line {i:03d} — the quick brown fox jumps over the lazy dog" for i in range(20))
    data = text.encode()

    print(f"input: {len(text.splitlines())} lines x {max(len(l) for l in text.splitlines())} cols"
          f" · best of {REPEATS} · frame pacing off\n")

    # startup: smallest possible unit of work
    tiny = b"x"
    rs_start = best_of([str(RUST), "--seed", "1", "--frame-rate", "0", "wipe"], tiny)
    py_start = best_of([sys.executable, "-m", "terminaltexteffects", "--frame-rate", "0", "wipe"], tiny)
    print(f"{'startup (1 char, wipe)':<22} rust {rs_start:7.1f} ms   python {py_start:8.1f} ms   {py_start/rs_start:5.1f}x\n")

    rows, ratios = [], []
    for e in effects():
        rs = best_of([str(RUST), "--seed", "1", "--frame-rate", "0", e], data)
        py = best_of([sys.executable, "-m", "terminaltexteffects", "--frame-rate", "0", e], data)
        n = frames_of(e, data)
        rows.append((e, rs, py, py / rs if rs else 0, n))
        if e not in CLOCK_BOUND:
            ratios.append(py / rs if rs else 0)

    rows.sort(key=lambda r: -r[3])
    print(f"{'effect':<17}{'rust ms':>9}{'python ms':>11}{'speedup':>9}{'frames':>8}{'rust fps':>10}")
    print("-" * 64)
    for e, rs, py, ratio, n in rows:
        mark = " *" if e in CLOCK_BOUND else ""
        fps = n / (rs / 1000) if rs else 0
        print(f"{e+mark:<17}{rs:9.1f}{py:11.1f}{ratio:8.1f}x{n:8d}{fps:10.0f}")

    ratios.sort()
    mid = ratios[len(ratios) // 2]
    print("-" * 64)
    print(f"median speedup (35 non-clock-bound effects): {mid:.1f}x"
          f"   range {min(ratios):.1f}x–{max(ratios):.1f}x")
    print("* clock-bound: gated on wall time, so the ratio reflects frames rendered, not time saved")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
