"""M0 parity dump: build a Terminal from CLI args, set every character in
character_by_input_coord visible, print the first frame. The Rust equivalent is
`ttfx --m0-dump`. Run with the pinned reference on PYTHONPATH.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "reference" / "tte"))

from terminaltexteffects.engine.terminal import Terminal, TerminalConfig  # noqa: E402
from terminaltexteffects.utils.exceptions import UnsupportedAnsiSequenceError  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser()
    TerminalConfig._populate_parser(parser)
    args = parser.parse_args()
    config = TerminalConfig._build_config(args)

    input_data = sys.stdin.read()
    if not input_data.strip():
        print("NO INPUT.")
        return 1

    try:
        terminal = Terminal(input_data, config)
    except UnsupportedAnsiSequenceError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1

    for character in terminal.character_by_input_coord.values():
        terminal.set_character_visibility(character, is_visible=True)
    print(terminal.get_formatted_output_string())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
