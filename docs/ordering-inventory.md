# Unordered-iteration inventory (plan.md §4.3)

Every site where upstream TTE iterates an unordered container (set) or relies
on dict insertion order, with the canonical deterministic order used by ttfx
and matched by the parity shim. Extend this file as effect ports discover new
sites — the parity harness surfaces them as first-divergence failures.

## Engine (sets → canonical order)

| Upstream site | Container | Canonical order | ttfx | shim patch |
|---|---|---|---|---|
| `BaseEffectIterator.update` (base_effect.py:92) | `active_characters` set | ascending `character_id` | `BTreeSet<CharId>` | sort snapshot by id |
| `Terminal._update_terminal_state` (terminal.py:1376) layer ties | `_visible_characters` set | `(layer, character_id)` | sort at render | sort key patched |
| `EffectCharacter.links` iteration in `BreadthFirst.step` (breadthfirst.py:90) | `links` set | ascending `character_id` | id-sorted `Vec` | `sorted(links, key=id)` |

## Effects (sets → canonical order)

| Upstream site | Container | Canonical order | ttfx | shim patch |
|---|---|---|---|---|
| `MiddleOutIterator.__next__` full-phase activation loop (effect_middleout.py:229) | rebuilt `active_characters` set | ascending `character_id` | iterate `BTreeSet<CharId>` | `MiddleOutIterator.__next__` sorts by id |

## Engine (dict insertion order = behavior)

| Upstream site | ttfx |
|---|---|
| `Motion.paths` / `Animation.scenes` values iteration (e.g. swarm chains `paths.values()`) | `OrderedMap` |
| `Terminal._input_colors_frequency` (equal-count ties in `get_input_colors`) | insertion-ordered `ColorFrequency` |
| `Gradient.build_coordinate_color_mapping` returned dict | `CoordColorMap` (insertion-ordered) |
| `character.neighbors` (north, east, south, west) | fixed-field struct in that order |
| `Scene.frame_index_map` | `Vec<usize>` tick map |
| `PrimsWeighted._pending_weighted_links` (defaultdict; `min` over keys) | `BTreeMap` (order-independent result) |
| distance buckets in `get_characters_grouped` CENTER/OUTSIDE | insertion-ordered vec of buckets |

## Effect-level sets (to be patched per effect during M3-M5 ports)

Known from the plan review; each gets its canonical order + shim patch when its
effect is ported:

- middleout (effect_middleout.py:229) — set iteration — DONE (see Effects table above)
- unstable (effect_unstable.py:332) — set iteration
- (audit each effect at port time; add rows here)

Audited, no patch needed (set iteration present but order-unobservable):

| Effect | Site | Why order-unobservable |
|---|---|---|
| slice | `for character in self.active_characters` at end of `build` (effect_slice.py) | only calls `set_character_visibility(True)`, a commutative per-character flag; render order is already canonicalized by the `_update_terminal_state` patch. ttfx iterates its `BTreeSet` (ascending id). |
| decrypt | `for char in self.active_characters` at the typing→decrypting transition (effect_decrypt.py:263) | only calls `activate_scene("fast_decrypt")`, which mutates each character alone; no SCENE_ACTIVATED handlers registered, so order is unobservable. ttfx iterates its `BTreeSet` (ascending id). |
| expand, scattered | none beyond `active_characters` membership | build loops iterate `get_characters()` lists; `active_characters` ticking covered by the `BaseEffectIterator.update` patch. |
