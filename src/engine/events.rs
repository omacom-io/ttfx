//! Event system, ported from engine/base_character.py EventHandler.
//!
//! Storage is plain data; dispatch lives on EngineCtx (engine/ctx.rs) so that
//! actions execute inline at the exact upstream emission points, reentrantly
//! (plan.md §4.2). Effect callbacks are (CallbackId, payload) routed through
//! EffectHooks::dispatch_callback — never closures in the arena.

use crate::engine::character::CharId;
use crate::utils::geometry::Coord;
use crate::utils::graphics::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Event {
    SegmentEntered,
    SegmentExited,
    PathActivated,
    PathComplete,
    PathHolding,
    SceneActivated,
    SceneComplete,
}

impl Event {
    #[inline]
    fn bit(self) -> u8 {
        1 << (self as u8)
    }
}

/// Waypoint identity for event keying: upstream Waypoint is a frozen dataclass
/// hashed/compared by ALL fields (id, coord, bezier controls) — two waypoints
/// with identical fields in different paths collide, faithfully.
///
/// Field order is load-bearing for speed, not for meaning: the derived
/// comparison short-circuits in declaration order, and `coord` rejects
/// non-matches with two integer compares instead of a string memcmp.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WaypointKey {
    pub coord: Coord,
    pub waypoint_id: std::rc::Rc<str>,
    pub bezier_control: Option<std::rc::Rc<[Coord]>>,
}

/// Event caller identity. Scene/Path compare by id (their upstream __eq__).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CallerKey {
    Scene(String),
    Path(String),
    Waypoint(WaypointKey),
}

/// A caller identity borrowed for the duration of a lookup. Emission sites
/// already hold the id they are firing for, so matching against the table costs
/// nothing — building an owned CallerKey per emission did.
#[derive(Debug, Clone, Copy)]
pub enum CallerRef<'a> {
    Scene(&'a str),
    Path(&'a str),
    Waypoint(&'a WaypointKey),
}

impl CallerKey {
    #[inline]
    fn matches(&self, caller: CallerRef<'_>) -> bool {
        match (self, caller) {
            (CallerKey::Scene(a), CallerRef::Scene(b)) => **a == *b,
            (CallerKey::Path(a), CallerRef::Path(b)) => **a == *b,
            (CallerKey::Waypoint(a), CallerRef::Waypoint(b)) => a == b,
            _ => false,
        }
    }
}

/// Typed payload values for effect callbacks (upstream Callback *args).
#[derive(Debug, Clone, PartialEq)]
pub enum CallbackValue {
    Int(i64),
    Float(f64),
    Str(String),
    Coord(Coord),
    Char(CharId),
    Color(Color),
}

/// An effect-defined callback: the id selects behavior inside the effect's
/// dispatch_callback; args are owned data captured at registration.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectCallback {
    pub id: u32,
    pub args: Vec<CallbackValue>,
}

/// A registered action with its resolved target (upstream resolves string ids
/// to objects at registration; Scene/Path equality is by id, so ids suffice).
#[derive(Debug, Clone, PartialEq)]
pub enum EventAction {
    ActivatePath(String),
    ActivateScene(String),
    DeactivatePath(Option<String>),
    DeactivateScene(Option<String>),
    ResetAppearance,
    SetLayer(i64),
    SetCoordinate(Coord),
    Callback(EffectCallback),
}

/// Per-character event table: insertion-ordered (event, caller) -> actions.
///
/// `subscribed` mirrors the table as a bitmask of registered event kinds. Most
/// characters register a handful of events while the engine emits thousands,
/// so callers test it first and skip building the (allocating) CallerKey when
/// nothing could match.
#[derive(Debug, Clone, Default)]
pub struct EventHandler {
    registered_events: Vec<((Event, CallerKey), Vec<EventAction>)>,
    subscribed: u8,
}

impl EventHandler {
    /// register_event with the duplicate check (upstream raises
    /// DuplicateEventRegistrationError). Caller/target id resolution and type
    /// validation happen in EngineCtx::register_event, which has arena access.
    pub fn push(&mut self, event: Event, caller: CallerKey, action: EventAction) -> Result<(), String> {
        let key = (event, caller);
        if let Some(entry) = self.registered_events.iter_mut().find(|(k, _)| *k == key) {
            if entry.1.contains(&action) {
                return Err(format!("duplicate event registration: {:?} {:?}", entry.0, action));
            }
            entry.1.push(action);
        } else {
            self.registered_events.push((key, vec![action]));
        }
        self.subscribed |= event.bit();
        Ok(())
    }

    /// True when at least one action is registered for this event kind, for any
    /// caller. A false answer means `actions_index` cannot match.
    #[inline]
    pub fn subscribes(&self, event: Event) -> bool {
        self.subscribed & event.bit() != 0
    }

    #[inline]
    pub fn actions_index(&self, event: Event, caller: CallerRef<'_>) -> Option<usize> {
        if !self.subscribes(event) {
            return None;
        }
        self.registered_events.iter().position(|((e, c), _)| *e == event && c.matches(caller))
    }

    #[inline]
    pub fn actions(&self, index: usize) -> &[EventAction] {
        &self.registered_events[index].1
    }

    pub fn clear(&mut self) {
        self.registered_events.clear();
        self.subscribed = 0;
    }
}
