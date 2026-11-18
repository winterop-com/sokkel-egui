//! sokkel-egui's graphs, the counterpart of `@winterop-com/sokkel-ui-graph`: node boxes and
//! the edges between them, laid out in ranks as elk's layered layout places them there, on a canvas
//! fitted to its width; and a network of dots and links laid out by forces, as its `NetworkGraph`.

pub mod canvas;
pub mod force;
pub mod layout;
pub mod network;
pub mod node;

pub use canvas::{GraphCanvas, GraphLink, GraphWords, fitted_zoom};
pub use force::{ForceBody, ForceSimulation};
pub use layout::{GraphDirection, GraphSpacing, PlacedGraph, layout};
pub use network::{KindColor, NetworkGraph, NetworkLink, NetworkNode, NetworkResponse, NetworkWords, dot_size};
pub use node::{GraphNode, NodeStatus};
