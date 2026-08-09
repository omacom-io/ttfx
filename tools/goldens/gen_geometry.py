"""Generate geometry goldens as canonical text lines. The Rust test
(tests/geometry_goldens.rs) regenerates the same lines and diffs.
Floats are emitted as f64 bit patterns for exactness.
"""

from __future__ import annotations

import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "reference" / "tte"))

from terminaltexteffects.utils import geometry  # noqa: E402
from terminaltexteffects.utils.geometry import Coord  # noqa: E402


def fbits(x: float) -> str:
    return struct.pack("<d", x).hex()


def coords(cs: list[Coord]) -> str:
    return ";".join(f"{c.column},{c.row}" for c in cs)


lines: list[str] = []

for radius in [1, 2, 3, 5, 8, 13, 20]:
    for limit in [0, 7, 100]:
        for unique in [True, False]:
            got = geometry.find_coords_on_circle(Coord(10, 10), radius, limit, unique=unique)
            lines.append(f"on_circle r={radius} l={limit} u={unique}: {coords(got)}")

for diameter in [1, 2, 3, 4, 7, 10, 15]:
    got = geometry.find_coords_in_circle(Coord(5, -3), diameter)
    lines.append(f"in_circle d={diameter}: {coords(got)}")

for distance in [0, 1, 2, 5]:
    lines.append(f"in_rect d={distance}: {coords(geometry.find_coords_in_rect(Coord(3, 4), distance))}")

for hw, hh in [(0, 3), (3, 0), (1, 1), (4, 2), (5, 7)]:
    lines.append(f"on_rect {hw},{hh}: {coords(geometry.find_coords_on_rect(Coord(0, 0), hw, hh))}")

for origin, target in [(Coord(0, 0), Coord(10, 5)), (Coord(3, 3), Coord(3, 3)), (Coord(-5, 2), Coord(7, -9))]:
    for offset in [0.0, 1.5, 4.0, 10.25, -2.0]:
        c = geometry.extrapolate_along_ray(origin, target, offset)
        lines.append(f"extrapolate {origin.column},{origin.row}->{target.column},{target.row}+{offset}: {c.column},{c.row}")

bezier_cases = [
    (Coord(0, 0), (Coord(5, 10),), Coord(10, 0)),
    (Coord(0, 0), (Coord(3, 8), Coord(7, -2)), Coord(12, 4)),
    (Coord(-4, -4), (Coord(0, 20), Coord(9, 9), Coord(-3, 2)), Coord(6, -6)),
]
for start, control, end in bezier_cases:
    pts = []
    for i in range(21):
        t = i / 20
        c = geometry.find_coord_on_bezier_curve(start, control, end, t)
        pts.append(c)
    lines.append(f"bezier {len(control)}cp: {coords(pts)}")
    lines.append(f"bezier_len {len(control)}cp: {fbits(geometry.find_length_of_bezier_curve(start, control, end))}")

line_pts = []
for i in range(-5, 26):
    t = i / 20
    line_pts.append(geometry.find_coord_on_line(Coord(-3, 7), Coord(14, -2), t))
lines.append(f"on_line: {coords(line_pts)}")

for double in [False, True]:
    v = geometry.find_length_of_line(Coord(1, 2), Coord(-7, 11), double_row_diff=double)
    lines.append(f"line_len double={double}: {fbits(v)}")

for coord in [Coord(1, 1), Coord(5, 3), Coord(10, 8), Coord(3, 8), Coord(10, 1)]:
    v = geometry.find_normalized_distance_from_center(1, 8, 1, 10, coord)
    lines.append(f"norm_dist {coord.column},{coord.row}: {fbits(v)}")

out = Path(__file__).resolve().parents[2] / "tests" / "fixtures" / "geometry_goldens.txt"
out.write_text("\n".join(lines) + "\n")
print(f"wrote {out} ({len(lines)} lines)")
