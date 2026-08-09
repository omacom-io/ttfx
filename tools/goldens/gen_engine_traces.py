"""Generate engine state-machine traces from the reference implementation.

Each scenario builds characters/paths/scenes, ticks the engine, and logs:
- every event emission (patched _handle_event), before its actions run
- per-tick character state (coord, layer, formatted visual, active path/scene)

The Rust test (tests/engine_traces.rs) replays identical scenarios and diffs.
No randomness is used, so no RNG shim is needed here.
"""

from __future__ import annotations

import sys
from pathlib import Path as P

sys.path.insert(0, str(P(__file__).resolve().parents[2] / "reference" / "tte"))

from terminaltexteffects.engine import animation as animation_mod  # noqa: E402
from terminaltexteffects.engine import motion as motion_mod  # noqa: E402
from terminaltexteffects.engine.base_character import EventHandler  # noqa: E402
from terminaltexteffects.engine.terminal import Terminal, TerminalConfig  # noqa: E402
from terminaltexteffects.utils import easing  # noqa: E402
from terminaltexteffects.utils.geometry import Coord  # noqa: E402
from terminaltexteffects.utils.graphics import Color, ColorPair, Gradient  # noqa: E402

LOG: list[str] = []


def caller_label(caller) -> str:
    if isinstance(caller, motion_mod.Path):
        return f"path:{caller.path_id}"
    if isinstance(caller, motion_mod.Waypoint):
        return f"wp:{caller.waypoint_id}"
    if isinstance(caller, animation_mod.Scene):
        return f"scene:{caller.scene_id}"
    return f"?:{caller}"


_orig_handle = EventHandler._handle_event


def logged_handle(self, event, caller):
    LOG.append(f"EVENT char={self.character.character_id} {event.name} caller={caller_label(caller)}")
    _orig_handle(self, event, caller)


EventHandler._handle_event = logged_handle


def esc(s: str) -> str:
    return s.replace("\x1b", "\\e")


def make_terminal() -> Terminal:
    config = TerminalConfig._build_config()
    config.canvas_width = 20
    config.canvas_height = 10
    config.ignore_terminal_dimensions = True
    config.frame_rate = 0
    return Terminal("abcdef\nghijkl", config)


def snapshot(tick: int, chars) -> None:
    for ch in chars:
        ap = ch.motion.active_path.path_id if ch.motion.active_path else "-"
        sc = ch.animation.active_scene.scene_id if ch.animation.active_scene else "-"
        LOG.append(
            f"tick={tick} char={ch.character_id} coord={ch.motion.current_coord.column},"
            f"{ch.motion.current_coord.row} layer={ch.layer} path={ap} scene={sc} "
            f"vis={esc(ch.animation.current_character_visual.formatted_symbol)} active={ch.is_active}"
        )


def run_ticks(chars, n: int, start: int = 0) -> None:
    active = set(chars)
    for tick in range(start, start + n):
        for ch in tuple(sorted(active, key=lambda c: c.character_id)):
            ch.tick()
        active -= {c for c in active if not c.is_active}
        snapshot(tick, chars)


def scenario_motion_basic() -> None:
    LOG.append("=== scenario_motion_basic ===")
    terminal = make_terminal()
    chars = terminal.get_characters()[:2]
    a, b = chars
    pa = a.motion.new_path(speed=0.7, path_id="pa")
    pa.new_waypoint(Coord(15, 8))
    pa.new_waypoint(Coord(18, 2), bezier_control=Coord(1, 1))
    pb = b.motion.new_path(speed=1.3, ease=easing.out_back, path_id="pb")
    pb.new_waypoint(Coord(3, 9))
    a.motion.activate_path(pa)
    b.motion.activate_path(pb)
    run_ticks(chars, 30)


def scenario_hold_and_loop() -> None:
    LOG.append("=== scenario_hold_and_loop ===")
    terminal = make_terminal()
    chars = terminal.get_characters()[:2]
    a, b = chars
    pa = a.motion.new_path(speed=2.0, hold_time=3, path_id="hold")
    pa.new_waypoint(Coord(10, 5))
    pb = b.motion.new_path(speed=2.0, loop=True, path_id="looper")
    pb.new_waypoint(Coord(6, 3))
    pb.new_waypoint(Coord(9, 6))
    a.motion.activate_path(pa)
    b.motion.activate_path(pb)
    run_ticks(chars, 20)


def scenario_chained_paths_and_events() -> None:
    LOG.append("=== scenario_chained_paths_and_events ===")
    terminal = make_terminal()
    chars = terminal.get_characters()[:1]
    a = chars[0]
    p1 = a.motion.new_path(speed=1.5, path_id="p1")
    p1.new_waypoint(Coord(5, 5))
    p2 = a.motion.new_path(speed=1.5, path_id="p2", layer=2)
    p2.new_waypoint(Coord(10, 2))
    p3 = a.motion.new_path(speed=1.5, path_id="p3")
    p3.new_waypoint(Coord(1, 1))
    a.motion.chain_paths([p1, p2, p3])
    a.event_handler.register_event(
        EventHandler.Event.PATH_COMPLETE, p3, EventHandler.Action.SET_COORDINATE, Coord(19, 9)
    )
    a.event_handler.register_event(EventHandler.Event.PATH_HOLDING, p1, EventHandler.Action.SET_LAYER, 7)
    a.motion.activate_path(p1)
    run_ticks(chars, 25)


def scenario_scenes() -> None:
    LOG.append("=== scenario_scenes ===")
    terminal = make_terminal()
    chars = terminal.get_characters()[:3]
    a, b, c = chars
    sa = a.animation.new_scene(scene_id="plain")
    sa.add_frame("X", 2, colors=ColorPair(fg=Color("ff0000")))
    sa.add_frame("Y", 3, colors=ColorPair(fg=Color("00ff00"), bg=Color(21)))
    sa.add_frame("Z", 1, bold=True)
    a.animation.activate_scene(sa)

    sb = b.animation.new_scene(scene_id="looping", is_looping=True)
    sb.add_frame("1", 2)
    sb.add_frame("2", 2)
    b.animation.activate_scene(sb)

    sc = c.animation.new_scene(scene_id="eased", ease=easing.in_out_cubic)
    grad = Gradient(Color("000000"), Color("ffffff"), steps=8)
    sc.apply_gradient_to_symbols(["*", "+", "o"], 2, fg_gradient=grad)
    c.animation.activate_scene(sc)
    run_ticks(chars, 24)


def scenario_synced_scene() -> None:
    LOG.append("=== scenario_synced_scene ===")
    terminal = make_terminal()
    chars = terminal.get_characters()[:2]
    a, b = chars
    for ch, sync, pid in ((a, animation_mod.Scene.SyncMetric.STEP, "sp"), (b, animation_mod.Scene.SyncMetric.DISTANCE, "dp")):
        path = ch.motion.new_path(speed=0.9, path_id=pid)
        path.new_waypoint(Coord(16, 9))
        path.new_waypoint(Coord(2, 2))
        scene = ch.animation.new_scene(sync=sync, scene_id=f"sync_{pid}")
        for i, sym in enumerate("abcdefgh"):
            scene.add_frame(sym, 1)
        ch.motion.activate_path(path)
        ch.animation.activate_scene(scene)
    run_ticks(chars, 30)


def scenario_scene_events_and_resume() -> None:
    LOG.append("=== scenario_scene_events_and_resume ===")
    terminal = make_terminal()
    chars = terminal.get_characters()[:1]
    a = chars[0]
    s1 = a.animation.new_scene(scene_id="s1")
    s1.add_frame("A", 3)
    s1.add_frame("B", 3)
    s2 = a.animation.new_scene(scene_id="s2")
    s2.add_frame("C", 2)
    a.event_handler.register_event(
        EventHandler.Event.SCENE_COMPLETE, s1, EventHandler.Action.ACTIVATE_SCENE, s2
    )
    path = a.motion.new_path(speed=1.0, path_id="mover")
    path.new_waypoint(Coord(8, 8))
    a.event_handler.register_event(
        EventHandler.Event.SCENE_COMPLETE, s2, EventHandler.Action.ACTIVATE_PATH, path
    )
    a.animation.activate_scene(s1)
    # partial play then re-activate to prove resume semantics
    run_ticks(chars, 2)
    a.animation.activate_scene(s1)
    run_ticks(chars, 20, start=2)


SCENARIOS = [
    scenario_motion_basic,
    scenario_hold_and_loop,
    scenario_chained_paths_and_events,
    scenario_scenes,
    scenario_synced_scene,
    scenario_scene_events_and_resume,
]

for scenario in SCENARIOS:
    scenario()

out = P(__file__).resolve().parents[2] / "tests" / "fixtures" / "engine_traces.txt"
out.write_text("\n".join(LOG) + "\n")
print(f"wrote {out} ({len(LOG)} lines)")
