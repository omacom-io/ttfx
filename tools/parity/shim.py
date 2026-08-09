"""Parity shim: makes the reference TTE deterministic and RNG-compatible with ttfx.

Install BEFORE importing terminaltexteffects:
    import shim; shim.install(seed)

Patches applied (plan.md §7):
- random.random/randint/randrange/choice/uniform/shuffle/seed -> xoshiro256++
  implementations bit-identical to src/utils/rng.rs (the parity contract; keep
  in lockstep with rng.rs).
- time.time/time.monotonic -> virtual clock (advance via shim.advance_frame),
  time.sleep -> no-op.
- BaseEffectIterator.update -> ticks in ascending character_id order.
- Terminal._update_terminal_state -> paints in (layer, character_id) order.
- BreadthFirst.step -> iterates links in ascending character_id order.
"""

from __future__ import annotations

MASK = (1 << 64) - 1


class Xoshiro256pp:
    def __init__(self, seed: int) -> None:
        self.seed(seed)

    def seed(self, seed: int) -> None:
        # SplitMix64 expansion, matching Rng::seeded
        sm = seed & MASK
        state = []
        for _ in range(4):
            sm = (sm + 0x9E3779B97F4A7C15) & MASK
            z = sm
            z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK
            z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK
            state.append(z ^ (z >> 31))
        self.s = state

    def next_u64(self) -> int:
        s = self.s
        result = (self._rotl((s[0] + s[3]) & MASK, 23) + s[0]) & MASK
        t = (s[1] << 17) & MASK
        s[2] ^= s[0]
        s[3] ^= s[1]
        s[1] ^= s[2]
        s[0] ^= s[3]
        s[2] ^= t
        s[3] = self._rotl(s[3], 45)
        return result

    @staticmethod
    def _rotl(x: int, k: int) -> int:
        return ((x << k) | (x >> (64 - k))) & MASK

    def random(self) -> float:
        return (self.next_u64() >> 11) * (1.0 / (1 << 53))

    def randbelow(self, n: int) -> int:
        assert n > 0
        bits = (n - 1).bit_length()
        shift = 64 - max(bits, 1)
        while True:
            r = self.next_u64() >> shift
            if r < n:
                return r

    def randint(self, a: int, b: int) -> int:
        return a + self.randbelow(b - a + 1)

    def randrange(self, a: int, b: int | None = None) -> int:
        if b is None:
            a, b = 0, a
        return a + self.randbelow(b - a)

    def choice(self, seq):
        return seq[self.randbelow(len(seq))]

    def uniform(self, a: float, b: float) -> float:
        return a + (b - a) * self.random()

    def shuffle(self, x) -> None:
        for i in reversed(range(1, len(x))):
            j = self.randbelow(i + 1)
            x[i], x[j] = x[j], x[i]


class VirtualClock:
    def __init__(self) -> None:
        self.now = 0.0
        self.dt = 1.0 / 60.0

    def advance_frame(self) -> None:
        self.now += self.dt


RNG = Xoshiro256pp(0)
CLOCK = VirtualClock()


def advance_frame() -> None:
    CLOCK.advance_frame()


def set_frame_rate(frame_rate: int) -> None:
    CLOCK.dt = 1.0 / frame_rate if frame_rate > 0 else 1.0 / 60.0


def install(seed: int) -> None:
    import random
    import time

    RNG.seed(seed)
    random.random = RNG.random
    random.randint = RNG.randint
    random.randrange = RNG.randrange
    random.choice = RNG.choice
    random.uniform = RNG.uniform
    random.shuffle = RNG.shuffle
    random.seed = RNG.seed

    time.time = lambda: CLOCK.now
    time.monotonic = lambda: CLOCK.now
    time.sleep = lambda _: None

    # determinism patches (canonical orders, docs/ordering-inventory.md)
    from terminaltexteffects.engine.base_effect import BaseEffectIterator
    from terminaltexteffects.engine.terminal import Terminal
    from terminaltexteffects.utils.spanningtree.algo.breadthfirst import BreadthFirst

    def update(self) -> None:
        for character in sorted(self.active_characters, key=lambda c: c.character_id):
            character.tick()
        self.active_characters -= {c for c in self.active_characters if not c.is_active}

    BaseEffectIterator.update = update

    def _update_terminal_state(self) -> None:
        rows = [[" " for _ in range(self.visible_right)] for _ in range(self.visible_top)]
        for character in sorted(self._visible_characters, key=lambda c: (c.layer, c.character_id)):
            row = character.motion.current_coord.row + self.canvas_row_offset
            column = character.motion.current_coord.column + self.canvas_column_offset
            if self.visible_bottom <= row <= self.visible_top and self.visible_left <= column <= self.visible_right:
                rows[row - 1][column - 1] = character.animation.current_character_visual.formatted_symbol
        self.terminal_state = ["".join(row) for row in rows]

    Terminal._update_terminal_state = _update_terminal_state

    _orig_bfs_step = BreadthFirst.step

    def bfs_step(self) -> None:
        self.explored_last_step.clear()
        if not self._frontier:
            self.complete = True
            return
        new_edges = []
        while self._frontier:
            position = self._frontier.pop(0)
            position_new_edges = [
                neighbor
                for neighbor in sorted(position.links, key=lambda c: c.character_id)
                if neighbor not in self._explored and neighbor not in self._frontier and neighbor not in new_edges
            ]
            new_edges.extend(position_new_edges)
            for character in position_new_edges:
                self._explored[character] = position
                self.explored_last_step.append(character)
                self.char_explore_order.append(character)
        self._frontier = new_edges

    BreadthFirst.step = bfs_step
    del _orig_bfs_step

    # middleout: __next__ iterates the rebuilt active_characters set to
    # activate the "full" path/scene — canonical ascending character_id
    # (docs/ordering-inventory.md, effect_middleout.py:229).
    from terminaltexteffects.effects.effect_middleout import MiddleOutIterator

    def middleout_next(self) -> str:
        if self.phase == "center" and not self.active_characters:
            self.phase = "full"
            self.active_characters = set(self.terminal.get_characters())
            for character in sorted(self.active_characters, key=lambda c: c.character_id):
                character.motion.activate_path("full")
                character.animation.activate_scene("full")
        if self.active_characters:
            self.update()
            return self.frame
        raise StopIteration

    MiddleOutIterator.__next__ = middleout_next

    # unstable: __next__ ticks the active_characters set directly in the
    # explosion (effect_unstable.py:332) and reassembly (effect_unstable.py:354)
    # phases — canonical ascending character_id (docs/ordering-inventory.md).
    from terminaltexteffects import Coord
    from terminaltexteffects.effects.effect_unstable import UnstableIterator

    def unstable_next(self) -> str:
        next_frame = None
        if self.phase == "rumble":
            if self._current_rumble_steps < self._max_rumble_steps:
                if self._current_rumble_steps > 30 and self._current_rumble_steps % self._rumble_mod_delay == 0:
                    row_offset = random.choice([-1, 0, 1])
                    column_offset = random.choice([-1, 0, 1])
                    for character in self.terminal.get_characters():
                        character.motion.set_coordinate(
                            Coord(
                                character.motion.current_coord.column + column_offset,
                                character.motion.current_coord.row + row_offset,
                            ),
                        )
                        character.animation.step_animation()
                    next_frame = self.frame
                    for character in self.terminal.get_characters():
                        character.motion.set_coordinate(self.jumbled_coords[character])
                    self._rumble_mod_delay -= 1
                    self._rumble_mod_delay = max(self._rumble_mod_delay, 1)
                else:
                    for character in self.terminal.get_characters():
                        character.animation.step_animation()
                    next_frame = self.frame

                self._current_rumble_steps += 1
            else:
                self.phase = "explosion"
                for character in self.terminal.get_characters():
                    character.motion.activate_path(character.motion.query_path("explosion"))
                self.active_characters = set(self.terminal.get_characters())

        if self.phase == "explosion":
            if self.active_characters:
                for character in sorted(self.active_characters, key=lambda c: c.character_id):
                    character.tick()
                self.active_characters = {
                    character
                    for character in self.active_characters
                    if character.motion.current_coord != character.motion.query_path("explosion").waypoints[0].coord
                }
                next_frame = self.frame

            elif self._explosion_hold_time:
                for character in sorted(self.active_characters, key=lambda c: c.character_id):
                    character.tick()
                self._explosion_hold_time -= 1
                next_frame = self.frame
            else:
                self.phase = "reassembly"
                for character in self.terminal.get_characters():
                    character.animation.activate_scene(character.animation.query_scene("final"))
                    self.active_characters.add(character)
                    character.motion.activate_path(character.motion.query_path("reassembly"))

        if self.phase == "reassembly" and self.active_characters:
            for character in sorted(self.active_characters, key=lambda c: c.character_id):
                character.tick()
            self.active_characters = {
                character
                for character in self.active_characters
                if character.motion.current_coord != character.motion.query_path("reassembly").waypoints[0].coord
                or not character.animation.active_scene_is_complete()
            }
            next_frame = self.frame

        if next_frame is not None:
            return next_frame
        raise StopIteration

    UnstableIterator.__next__ = unstable_next
