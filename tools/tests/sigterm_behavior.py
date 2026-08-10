"""SIGTERM cleanup behavior, driven on a real pty.

Interactive output must restore the cursor before exiting. Redirected output
keeps the default SIGTERM behavior and must not gain teardown bytes.
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
ARGS = ["--frame-rate", "30", "colorshift", "--cycles", "100"]


def spawn(stdout_pipe: bool):
    stdin_read, stdin_write = os.pipe()
    stdout_read, stdout_write = os.pipe() if stdout_pipe else (None, None)
    pid, tty = pty.fork()
    if pid == 0:
        os.close(stdin_write)
        os.dup2(stdin_read, 0)
        os.close(stdin_read)
        if stdout_pipe:
            os.close(stdout_read)
            os.dup2(stdout_write, 1)
            os.close(stdout_write)
        os.execv(BIN, [BIN] + ARGS)
        os._exit(127)
    os.close(stdin_read)
    if stdout_pipe:
        os.close(stdout_write)
    fcntl.ioctl(tty, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
    os.write(stdin_write, b"hello\n")
    os.close(stdin_write)
    return pid, tty, stdout_read


def run(stdout_pipe: bool):
    pid, tty, stdout_read = spawn(stdout_pipe)
    source = stdout_read if stdout_pipe else tty
    captured = bytearray()
    deadline = time.monotonic() + 2
    while HIDE not in captured and time.monotonic() < deadline:
        ready, _, _ = select.select([source], [], [], 0.02)
        if ready:
            captured.extend(os.read(source, 65536))

    os.kill(pid, signal.SIGTERM)
    status = None
    deadline = time.monotonic() + 2
    while time.monotonic() < deadline:
        ready, _, _ = select.select([source], [], [], 0.02)
        if ready:
            try:
                chunk = os.read(source, 65536)
            except OSError:
                chunk = b""
            captured.extend(chunk)
        done, candidate = os.waitpid(pid, os.WNOHANG)
        if done:
            status = candidate
            break
    if status is None:
        os.kill(pid, signal.SIGKILL)
        _, status = os.waitpid(pid, 0)
    # The child may exit just before its final tty bytes become readable.
    deadline = time.monotonic() + 0.1
    while time.monotonic() < deadline:
        ready, _, _ = select.select([source], [], [], 0.01)
        if not ready:
            break
        try:
            chunk = os.read(source, 65536)
        except OSError:
            break
        if not chunk:
            break
        captured.extend(chunk)
    os.close(source)
    if stdout_pipe:
        os.close(tty)
    return os.waitstatus_to_exitcode(status), bytes(captured)


def main() -> int:
    tty_status, tty_output = run(False)
    pipe_status, pipe_output = run(True)
    checks = [
        ("tty exits 143", tty_status == 143),
        ("tty restores cursor", tty_output.count(HIDE) == 1 and tty_output.count(SHOW) == 1),
        ("pipe keeps default SIGTERM", pipe_status == -signal.SIGTERM),
        ("pipe gets no teardown", SHOW not in pipe_output),
    ]
    for label, passed in checks:
        print(f"  {'ok  ' if passed else 'FAIL'} {label}")
    failures = sum(not passed for _, passed in checks)
    print(f"\nSIGTERM behavior: {'all checks passed' if not failures else f'{failures} failed'}")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
