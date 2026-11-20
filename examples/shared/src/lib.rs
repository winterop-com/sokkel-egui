//! The counterpart of sokkel-ui's `examples/shared`: what more than one of Tidevann's examples
//! uses. `clips` holds the procedural clips the video and cut examples play: a scene painted at any
//! second, and the sound made under it, sample by sample. `backend` is the fake backend the web
//! examples share (`tidevann-shared/backend`): its collections, held clock and where the changes
//! stand. `js` holds the stand-ins for what the browser does that the examples' web tests hold to
//! the web's answers: numbers, texts, collation, dates, JSON and URIs as the web reads and writes
//! them.

pub mod backend;
pub mod clips;
pub mod js;
