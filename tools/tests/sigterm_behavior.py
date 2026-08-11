"""SIGTERM teardown behavior, driven on a real pty.

A supervisor killing an animation must not leave the cursor hidden. On a tty
ttfx restores it and then dies from the signal; a redirected stream keeps the
default action and gains no teardown bytes. Either way the parent sees a child
terminated by SIGTERM. Only a real tty shows any of this, so this spawns the
built binary on a pty and asserts on the emitted byte stream.

Usage: sigterm_behavior.py [path-to-ttfx]
"""

from __future__ import annotations

import fcntl
import os
import pty
import select
import signal
import struct
import sys
import termios
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BIN = sys.argv[1] if len(sys.argv) > 1 else str(ROOT / "target/release/ttfx")
HIDE, SHOW = b"\x1b[?25l", b"\x1b[?25h"
# Long enough that the signal always lands mid-animation.
ARGS = ["--frame-rate", "30", "colorshift", "--cycles", "100"]


def spawn(stdout_pipe: bool):
    """Fork onto a pty; stdin is always a pipe, stdout optionally one too."""
    sin_r, sin_w = os.pipe()
    out_r, out_w = os.pipe() if stdout_pipe else (None, None)
    pid, fd = pty.fork()
    if pid == 0:
        os.close(sin_w)
        os.dup2(sin_r, 0)
        os.close(sin_r)
        if stdout_pipe:
            os.close(out_r)
            os.dup2(out_w, 1)
            os.close(out_w)
        # COLUMNS/LINES would win over the tty we just sized.
        env = {k: v for k, v in os.environ.items() if k not in ("COLUMNS", "LINES")}
        os.execve(BIN, [BIN] + ARGS, env)
        os._exit(127)
    os.close(sin_r)
    if stdout_pipe:
        os.close(out_w)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
    os.write(sin_w, b"hello\n")
    os.close(sin_w)
    return pid, fd, out_r


def drain(source: int, captured: bytearray, until: bytes | None = None) -> None:
    """Read until `until` shows up, or with no needle until the source ends."""
    deadline = time.monotonic() + 2.0
    while time.monotonic() < deadline:
        if until is not None and until in captured:
            return
        if not select.select([source], [], [], 0.02)[0]:
            continue
        try:
            chunk = os.read(source, 65536)
        except OSError:  # the pty master reports EIO once the child is gone
            return
        if not chunk:
            return
        captured.extend(chunk)


def reap(pid: int) -> int:
    deadline = time.monotonic() + 2.0
    while time.monotonic() < deadline:
        done, status = os.waitpid(pid, os.WNOHANG)
        if done:
            return status
        time.sleep(0.01)
    os.kill(pid, signal.SIGKILL)
    return os.waitpid(pid, 0)[1]


def run(stdout_pipe: bool):
    pid, fd, out_r = spawn(stdout_pipe)
    source = out_r if stdout_pipe else fd
    captured = bytearray()
    drain(source, captured, until=HIDE)
    os.kill(pid, signal.SIGTERM)
    # The stream ends when the child does, so this also waits for the exit.
    drain(source, captured)
    status = reap(pid)
    os.close(source)
    if stdout_pipe:
        os.close(fd)
    return status, bytes(captured)


def killed_by_sigterm(status: int) -> bool:
    return os.WIFSIGNALED(status) and os.WTERMSIG(status) == signal.SIGTERM


def main() -> int:
    tty_status, tty_output = run(False)
    pipe_status, pipe_output = run(True)
    checks = [
        ("tty dies from SIGTERM", killed_by_sigterm(tty_status)),
        ("tty restores the cursor", tty_output.count(HIDE) == 1 and tty_output.count(SHOW) == 1),
        ("pipe dies from SIGTERM", killed_by_sigterm(pipe_status)),
        ("pipe gets no teardown", SHOW not in pipe_output),
    ]
    for label, passed in checks:
        print(f"  {'ok  ' if passed else 'FAIL'} {label}")
    failures = sum(not passed for _, passed in checks)
    print(f"\nsigterm behavior: {'all checks passed' if not failures else f'{failures} failed'}")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
