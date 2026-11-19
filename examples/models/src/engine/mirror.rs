//! A collection's rows held for the engines, as the web's `engine/mirror.ts` holds them: the rows
//! as the backend last told them, which the engines read at once and write through the backend.
//! Here each collection keeps them itself (`Collection::seen`), every mirror of one collection
//! hearing the same answers; `Backend::new` lists them as the web's engines do as they load.

pub use crate::data::backend::Collection as Mirror;
