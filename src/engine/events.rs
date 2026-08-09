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

/// Waypoint identity for event keying: upstream Waypoint is a frozen dataclass
/// hashed/compared by ALL fields (id, coord, bezier controls) — two waypoints
/// with identical fields in different paths collide, faithfully.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WaypointKey {
    pub waypoint_id: String,
    pub coord: Coord,
    pub bezier_control: Option<Vec<Coord>>,
}

/// Event caller identity. Scene/Path compare by id (their upstream __eq__).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CallerKey {
    Scene(String),
    Path(String),
    Waypoint(WaypointKey),
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
#[derive(Debug, Clone, Default)]
pub struct EventHandler {
    pub registered_events: Vec<((Event, CallerKey), Vec<EventAction>)>,
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
        Ok(())
    }

    pub fn actions_index(&self, event: Event, caller: &CallerKey) -> Option<usize> {
        self.registered_events.iter().position(|((e, c), _)| *e == event && c == caller)
    }
}
