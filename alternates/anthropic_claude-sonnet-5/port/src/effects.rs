//! Static effect registry.
//!
//! Mirrors upstream's `pkgutil.iter_modules` discovery over
//! `terminaltexteffects.effects` (see `terminaltexteffects/__main__.py`), but
//! since Rust has no runtime module scanning, effects register themselves
//! into a static table at startup instead. Each ported effect (one file per
//! Python `effect_*.py`, per plan.md §6) will add a `register()` call inside
//! `register_builtin_effects` below as it lands; the table starts empty.
//!
//! This also stands in for the `Effect` trait sketched in plan.md §4.4:
//!
//! 