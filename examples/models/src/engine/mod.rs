//! The engines, as the web's `engine/` has them: the simulated model (`generate`), answers as
//! they stream (`stream`), the chat and the comparison (`chat`), the pulls (`downloads`) and the
//! runner (`runner`), all on the backend's clock.

pub mod chat;
pub mod downloads;
pub mod generate;
pub mod mirror;
pub mod runner;
pub mod stream;
