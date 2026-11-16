//! sokkel-egui's graphs, the counterpart of `@winterop-com/sokkel-ui-graph`: node boxes and
//! the edges between them, laid out in ranks as elk's layered layout places them there, on a canvas
//! fitted to its width.

pub mod canvas;
pub mod layout;
pub mod node;

pub use canvas::{GraphCanvas, GraphLink, GraphWords, fitted_zoom};
pub use layout::{GraphDirection, GraphSpacing, PlacedGraph, layout};
pub use node::{GraphNode, NodeStatus};
