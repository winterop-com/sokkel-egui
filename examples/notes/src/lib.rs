//! sokkel-ui's notes example in egui: Tidevann's vault of linked notes. The data layer so far, one
//! to one with the web's: the model, the seeded vault, links, tags and backlinks, search, and the
//! backend every change goes through.

pub mod backend;
pub mod model;
pub mod search;
pub mod seed;
pub mod vault;
pub mod wiki_links;
