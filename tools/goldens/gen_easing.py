"""Generate easing goldens: for each easing function, 1001 f64 samples over
[0, 1], written as little-endian binary. Rust test compares bit patterns.
Order must match EASING_GOLDEN_ORDER in tests/easing_goldens.rs.
"""

from __future__ import annotations

import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "reference" / "tte"))

from terminaltexteffects.utils import easing  # noqa: E402

NAMED = [
    easing.linear, easing.in_sine, easing.out_sine, easing.in_out_sine,
    easing.in_quad, easing.out_quad, easing.in_out_quad,
    easing.in_cubic, easing.out_cubic, easing.in_out_cubic,
    easing.in_quart, easing.out_quart, easing.in_out_quart,
    easing.in_quint, easing.out_quint, easing.in_out_quint,
    easing.in_expo, easing.out_expo, easing.in_out_expo,
    easing.in_circ, easing.out_circ, easing.in_out_circ,
    easing.in_back, easing.out_back, easing.in_out_back,
    easing.in_elastic, easing.out_elastic, easing.in_out_elastic,
    easing.in_bounce, easing.out_bounce, easing.in_out_bounce,
]

BEZIERS = [(0.25, 0.1, 0.25, 1.0), (0.42, 0.0, 0.58, 1.0), (0.68, -0.55, 0.265, 1.55)]

out = Path(__file__).resolve().parents[2] / "tests" / "fixtures" / "easing_goldens.bin"
out.parent.mkdir(parents=True, exist_ok=True)

with out.open("wb") as f:
    for fn in NAMED + [easing.make_easing(*b) for b in BEZIERS]:
        for i in range(1001):
            f.write(struct.pack("<d", fn(i / 1000)))

print(f"wrote {out} ({out.stat().st_size} bytes)")
