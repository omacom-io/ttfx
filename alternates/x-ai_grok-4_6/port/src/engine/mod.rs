pub mod animation;
pub mod canvas;
pub mod character;
pub mod motion;
pub mod terminal;

pub use animation::{Animation, CharacterVisual, Frame, Scene};
pub use canvas::{Canvas, Cell};
pub use character::{CharacterId, EffectCharacter, Neighbors};
pub use motion::{Motion, Path, Segment, Waypoint};
pub use terminal::{Anchor, ExistingColorHandling, Terminal, TerminalConfig, TtyWriter};
