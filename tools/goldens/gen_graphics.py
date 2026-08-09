"""Generate gradient/color goldens as canonical text lines."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "reference" / "tte"))

from terminaltexteffects.utils.graphics import Color, Gradient, shift_color_towards  # noqa: E402

lines: list[str] = []

grad_cases = [
    (["8A008A", "00D1FF", "FFFFFF"], (12,), False),
    (["8A008A", "00D1FF", "FFFFFF"], (6, 3), False),
    (["ffffff", "000000"], (10,), False),
    (["000000", "ffffff"], (7,), False),
    (["ff0000", "00ff00", "0000ff"], (5,), True),
    (["123456"], (4,), False),
    (["ff5733", "33ff57", "5733ff", "f0f0f0"], (3, 9), False),
    (["0a0b0c", "f1e2d3"], (1,), False),
]
for stops, steps, loop in grad_cases:
    g = Gradient(*[Color(s) for s in stops], steps=steps, loop=loop)
    lines.append(f"grad {'+'.join(stops)} s={steps} loop={loop}: {';'.join(c.rgb_color for c in g.spectrum)}")

g = Gradient(Color("8A008A"), Color("00D1FF"), Color("FFFFFF"), steps=12)
for i in range(21):
    f = i / 20
    lines.append(f"frac {f}: {g.get_color_at_fraction(f).rgb_color}")

for direction in ["VERTICAL", "HORIZONTAL", "RADIAL", "DIAGONAL"]:
    mapping = g.build_coordinate_color_mapping(1, 5, 1, 8, getattr(Gradient.Direction, direction))
    entries = ";".join(f"{c.column},{c.row}={col.rgb_color}" for c, col in mapping.items())
    lines.append(f"mapping {direction}: {entries}")
    mapping = g.build_coordinate_color_mapping(2, 6, 3, 9, getattr(Gradient.Direction, direction))
    entries = ";".join(f"{c.column},{c.row}={col.rgb_color}" for c, col in mapping.items())
    lines.append(f"mapping_offset {direction}: {entries}")

for factor in [0.0, 0.1, 0.25, 0.5, 0.75, 0.99, 1.0]:
    c = shift_color_towards(Color("ff8040"), Color("103050"), factor)
    lines.append(f"shift {factor}: {c.rgb_color}")

out = Path(__file__).resolve().parents[2] / "tests" / "fixtures" / "graphics_goldens.txt"
out.write_text("\n".join(lines) + "\n")
print(f"wrote {out} ({len(lines)} lines)")
