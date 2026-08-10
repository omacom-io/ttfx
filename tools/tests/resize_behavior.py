"""Resize-restart behavior, driven on a real pty.

The restart-on-resize path is signal-driven and tty-dependent, so none of the
other suites can see it: parity runs go through --parity-dump, and the CLI
corpus never allocates a terminal. This spawns the built binary on a pty,
drives TIOCSWINSZ, and asserts on the emitted byte stream.

Each prep_canvas emits exactly one hide-cursor, so counting those counts runs.

Usage: resize_behavior.py [path-to-ttfx]
"""

from __future__ import annotations

import fcntl
import os
import pty
import select
import struct
import sys
import termios
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BIN = sys.argv[1] if len(sys.argv) > 1 else str(ROOT / "target/release/ttfx")
HIDE, SHOW = b"\x1b[?25l", b"\x1b[?25h"
MAX_BYTES = 8 << 20
TEXT = b"hello world\nsecond line"


def set_size(fd: int, cols: int, rows: int) -> None:
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))


def child_env() -> dict[str, str]:
    # COLUMNS/LINES would win over the tty and make every resize invisible.
    return {k: v for k, v in os.environ.items() if k not in ("COLUMNS", "LINES")}


def spawn(args, stdout_pipe: bool):
    """Fork onto a pty; stdin is always a pipe, stdout optionally one too."""
    sin_r, sin_w = os.pipe()
    out_r, out_w = (os.pipe() if stdout_pipe else (None, None))
    pid, fd = pty.fork()
    if pid == 0:
        os.close(sin_w)
        os.dup2(sin_r, 0)
        os.close(sin_r)
        if stdout_pipe:
            os.close(out_r)
            os.dup2(out_w, 1)
            os.close(out_w)
        os.execve(BIN, [BIN] + args, child_env())
        os._exit(127)
    os.close(sin_r)
    if stdout_pipe:
        os.close(out_w)
    return pid, fd, sin_w, out_r


def drive(args, resizes=(), cols=80, rows=24, first_delay=0.30, gap=0.05,
          budget=8.0, stdout_pipe=False, slow=False):
    """Run to completion (or budget), applying `resizes`; return the stream."""
    pid, fd, sin_w, out_r = spawn(args, stdout_pipe)
    set_size(fd, cols, rows)
    os.write(sin_w, TEXT)
    os.close(sin_w)

    source = out_r if stdout_pipe else fd
    captured = bytearray()
    start = time.time()
    applied = False
    while time.time() - start < budget and len(captured) < MAX_BYTES:
        if not applied and time.time() - start >= first_delay:
            for size in resizes:
                set_size(fd, *size)
                time.sleep(gap)
            applied = True
        ready, _, _ = select.select([source], [], [], 0.02)
        if ready:
            try:
                chunk = os.read(source, 256 if slow else 65536)
            except OSError:
                break
            if not chunk:
                break
            captured.extend(chunk)
        elif not stdout_pipe and os.waitpid(pid, os.WNOHANG)[0] == pid:
            break
        if slow:
            time.sleep(0.02)
    for closer in (lambda: os.kill(pid, 9), lambda: os.waitpid(pid, 0)):
        try:
            closer()
        except (ProcessLookupError, ChildProcessError):
            pass
    os.close(fd)
    if stdout_pipe:
        os.close(out_r)
    return bytes(captured)


def runs_in(stream: bytes) -> int:
    return stream.count(HIDE)


def main() -> int:
    failures = 0

    def check(label, got, want):
        nonlocal failures
        ok = got == want
        failures += not ok
        print(f"  {'ok  ' if ok else 'FAIL'} {label}: got {got}, want {want}")

    # A slow consumer keeps the process alive across the resize; a file sink
    # drains instantly and the animation would finish before the signal lands.
    print("stdout piped to a slow consumer, window resized")
    piped = ["--seed", "1", "--canvas-width", "0", "pour"]
    quiet = drive(piped, stdout_pipe=True, slow=True, first_delay=0.5, budget=6.0)
    noisy = drive(piped, resizes=[(40, 24)], stdout_pipe=True, slow=True, first_delay=0.5, budget=6.0)
    check("runs without a resize", runs_in(quiet), 1)
    check("runs with a resize", runs_in(noisy), 1)
    check("bytes match the undisturbed run", len(noisy), len(quiet))

    print("tty, resize that cannot move a cell")
    check("runs", runs_in(drive(["--seed", "1", "pour"], resizes=[(100, 30)])), 1)

    print("tty, resize that changes the canvas")
    changed = drive(["--seed", "1", "pour"], resizes=[(8, 24)])
    check("runs", runs_in(changed), 2)

    print("tty, burst of resizes during a drag")
    burst = drive(["--seed", "1", "pour"],
                  resizes=[(9, 24), (8, 24), (7, 24), (6, 24), (7, 24), (8, 24)], gap=0.02)
    rebuilds = runs_in(burst)
    ok = rebuilds <= 3
    failures += not ok
    print(f"  {'ok  ' if ok else 'FAIL'} rebuilds for a 6-step drag: {rebuilds}, want <= 3")

    print("tty, cursor is not shown between runs")
    check("show-cursor before the rebuild", changed[: changed.rfind(HIDE)].count(SHOW), 0)

    print(f"\nresize behavior: {'all checks passed' if not failures else f'{failures} failed'}")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
