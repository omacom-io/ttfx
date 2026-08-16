"""Losing the terminal mid-animation, driven on a real pty.

When the terminal an animation is drawing on goes away — the screensaver's
window killed at lock, an emulator exiting — every write from here on fails
with EIO. That is the display ending, not a failure of the run: ttfx must stop
quietly, say nothing about it, and above all not abort. Reporting it was the
whole bug (basecamp/omarchy#6762): `eprintln!` panics when stderr is the dead
terminal too, and a release build aborts on panic, so every idle lock left a
core dump behind.

Only a real pty shows this, so this spawns the built binary with its stdout on
one and closes the master out from under it.

Usage: terminal_close_behavior.py [path-to-ttfx]
"""

from __future__ import annotations

import fcntl
import os
import select
import signal
import struct
import sys
import termios
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BIN = sys.argv[1] if len(sys.argv) > 1 else str(ROOT / "target/release/ttfx")
HIDE = b"\x1b[?25l"
# Long enough that the terminal always closes mid-animation.
ARGS = ["--frame-rate", "30", "colorshift", "--cycles", "100"]


def spawn(stderr_pipe: bool):
    """Fork with stdout on a pty; stderr optionally on a pipe we can read.

    Deliberately no setsid: with the pty as a controlling terminal the kernel
    also sends SIGHUP when the master closes, and the race between that and
    the failing write would decide which path the child took. Without one,
    only the write error is left — the path under test.
    """
    sin_r, sin_w = os.pipe()
    err_r, err_w = os.pipe() if stderr_pipe else (None, None)
    master, slave = os.openpty()
    pid = os.fork()
    if pid == 0:
        os.close(master)
        os.close(sin_w)
        os.dup2(sin_r, 0)
        os.dup2(slave, 1)
        os.dup2(err_w if stderr_pipe else slave, 2)
        os.close(slave)
        # COLUMNS/LINES would win over the tty we just sized.
        env = {k: v for k, v in os.environ.items() if k not in ("COLUMNS", "LINES")}
        os.execve(BIN, [BIN] + ARGS, env)
        os._exit(127)
    os.close(slave)
    os.close(sin_r)
    if stderr_pipe:
        os.close(err_w)
    fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
    os.write(sin_w, b"hello\n")
    os.close(sin_w)
    return pid, master, err_r


def drain(source: int, until: bytes | None = None) -> bytes:
    """Read until `until` shows up, or with no needle until the source ends."""
    captured = bytearray()
    deadline = time.monotonic() + 2.0
    while time.monotonic() < deadline:
        if until is not None and until in captured:
            break
        if not select.select([source], [], [], 0.02)[0]:
            continue
        try:
            chunk = os.read(source, 65536)
        except OSError:  # the pty master reports EIO once the child is gone
            break
        if not chunk:
            break
        captured.extend(chunk)
    return bytes(captured)


def reap(pid: int) -> int | None:
    """Wait out the exit; None means it is still running and had to be killed."""
    deadline = time.monotonic() + 2.0
    while time.monotonic() < deadline:
        done, status = os.waitpid(pid, os.WNOHANG)
        if done:
            return status
        time.sleep(0.01)
    os.kill(pid, signal.SIGKILL)
    os.waitpid(pid, 0)
    return None


def run(stderr_pipe: bool):
    pid, master, err_r = spawn(stderr_pipe)
    drain(master, until=HIDE)
    os.close(master)  # the terminal is gone; the next frame hits EIO
    status = reap(pid)
    stderr = drain(err_r) if stderr_pipe else b""
    if stderr_pipe:
        os.close(err_r)
    return status, stderr


def exited_quietly(status: int | None) -> bool:
    return status is not None and os.WIFEXITED(status) and os.WEXITSTATUS(status) == 0


def aborted(status: int | None) -> bool:
    return status is not None and os.WIFSIGNALED(status) and os.WTERMSIG(status) == signal.SIGABRT


def main() -> int:
    if sys.platform != "linux":
        # A pty slave whose master is closed only reliably reports EIO here.
        print("terminal close behavior: skipped (linux only)")
        return 0
    pipe_status, pipe_stderr = run(True)
    tty_status, _ = run(False)
    checks = [
        ("a closed terminal ends the run", exited_quietly(pipe_status)),
        ("nothing is reported about it", pipe_stderr == b""),
        ("no abort with stderr on the dead terminal", not aborted(tty_status)),
        ("that run ends too", tty_status is not None),
    ]
    for label, passed in checks:
        print(f"  {'ok  ' if passed else 'FAIL'} {label}")
    failures = sum(not passed for _, passed in checks)
    if failures and pipe_stderr:
        print(f"\nstderr: {pipe_stderr.decode(errors='replace').strip()}")
    print(f"\nterminal close behavior: {'all checks passed' if not failures else f'{failures} failed'}")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
