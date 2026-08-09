"""Parity frame dumper for the reference TTE.

Usage: dump.py --seed N [--max-frames N] [terminal args] <effect> [effect args]
Reads input from stdin, runs the effect deterministically (shim installed),
writes each frame to stdout as b"%d\n" % len(frame) + frame + b"\n".
"""

from __future__ import annotations

import argparse
import importlib
import pkgutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[0]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "reference" / "tte"))

import shim  # noqa: E402

# peel off --seed / --max-frames before the real parser sees argv
argv = sys.argv[1:]
seed = 0
max_frames = None
while argv and argv[0] in ("--seed", "--max-frames"):
    flag = argv.pop(0)
    value = int(argv.pop(0))
    if flag == "--seed":
        seed = value
    else:
        max_frames = value

shim.install(seed)

import terminaltexteffects.effects  # noqa: E402
from terminaltexteffects.engine.terminal import Terminal, TerminalConfig  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser(prog="tte-parity-dump")
    TerminalConfig._populate_parser(parser)
    subparsers = parser.add_subparsers(title="effect", dest="effect")
    effects = {}
    for module_info in pkgutil.iter_modules(terminaltexteffects.effects.__path__):
        module = importlib.import_module(f"terminaltexteffects.effects.{module_info.name}")
        if hasattr(module, "get_effect_resources"):
            name, effect_cls, config_cls = module.get_effect_resources()
            effects[name] = (effect_cls, config_cls)
            config_cls._populate_parser(subparsers)

    args = parser.parse_args(argv)
    if not args.effect:
        print("no effect given", file=sys.stderr)
        return 2

    terminal_config = TerminalConfig._build_config(args)
    shim.set_frame_rate(terminal_config.frame_rate)
    effect_cls, config_cls = effects[args.effect]
    effect_config = config_cls._build_config(args)

    input_data = sys.stdin.read()
    effect = effect_cls(input_data, effect_config, terminal_config)

    out = sys.stdout.buffer
    count = 0
    for frame in effect:
        data = frame.encode("utf-8")
        out.write(b"%d\n" % len(data))
        out.write(data)
        out.write(b"\n")
        shim.advance_frame()
        count += 1
        if max_frames is not None and count >= max_frames:
            break
    out.flush()
    print(f"frames={count}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
