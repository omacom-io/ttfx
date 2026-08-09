"""Full tty-path runner for the reference TTE: replicates __main__.py's
`with effect.terminal_output() as terminal: for frame in effect: terminal.print(frame)`
with the shim installed, so the COMPLETE output byte stream (canvas prep, DEC
save/restore, per-frame cursor moves, teardown) can be compared against a real
`ttfx --seed N <effect>` run. Use --frame-rate 0 so pacing is disabled on both
sides.

Usage: tty_run.py --seed N [terminal args] <effect> [effect args] < input > bytes
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

argv = sys.argv[1:]
seed = 0
if argv and argv[0] == "--seed":
    argv.pop(0)
    seed = int(argv.pop(0))

shim.install(seed)

import terminaltexteffects.effects  # noqa: E402
from terminaltexteffects.engine.terminal import TerminalConfig  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser(prog="tte-tty-run")
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
    terminal_config = TerminalConfig._build_config(args)
    shim.set_frame_rate(terminal_config.frame_rate)
    effect_cls, config_cls = effects[args.effect]
    effect_config = config_cls._build_config(args)

    input_data = sys.stdin.read()
    effect = effect_cls(input_data, effect_config, terminal_config)

    with effect.terminal_output() as terminal:
        for frame in effect:
            terminal.print(frame)
            shim.advance_frame()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
