//! The clips: `scenes` paints a scene of Tidevann's world at any second, `sound` makes the sound
//! under it, both the same every time for the same spec.

pub mod scenes;
pub mod sound;

pub use scenes::{SceneKind, SceneSpec, Seeded, draw_frame};
pub use sound::{SOUND_RATE, synthesize};
