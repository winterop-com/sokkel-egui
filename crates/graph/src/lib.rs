//! sokkel-egui's graphs, the counterpart of `@winterop-com/sokkel-ui-graph`: node boxes and
//! the edges between them, laid out in ranks as elk's layered layout places them there, on a canvas
//! fitted to its width or a stage that edits them; a pipeline's steps and a run's, the parts an
//! editor is built from (a toolbar, the view's buttons, overlays, a block picker, undo); and a
//! network of dots and links laid out by forces, as its `NetworkGraph`.

pub mod blocks;
pub mod canvas;
pub mod cycles;
pub mod force;
pub mod history;
pub mod layout;
pub mod network;
pub mod node;
pub mod overlay;
pub mod picker;
pub mod pipeline;
pub mod run;
mod stage;
pub mod toolbar;
pub mod view_buttons;

pub use blocks::{BlockBadge, BlockGroup, GraphDragItem, PipelineBlock, block_matches, group_blocks};
pub use canvas::{
    ConnectCheck, EdgeMotion, EdgeTone, GraphCanvas, GraphControls, GraphEdgeRef, GraphEditing, GraphEvent, GraphFrame,
    GraphLink, GraphMenu, GraphPointer, GraphResponse, GraphView, GraphWords, NodeMenuFn, PaneMenuFn, fit_direction,
    fit_floor, fit_view, fitted_zoom, rough_route,
};
pub use cycles::{StepNeeds, cycle_path, would_cycle};
pub use force::{ForceBody, ForceSimulation};
pub use history::{GraphHistory, HistoryKey, history_key, history_keys};
pub use layout::{GraphDirection, GraphSpacing, PlacedGraph, layout};
pub use network::{KindColor, NetworkGraph, NetworkLink, NetworkNode, NetworkResponse, NetworkWords, dot_size};
pub use node::{GraphNode, IssueTone, NodeIssue, NodeStatus, worst_issue};
pub use overlay::{
    GraphAnchor, GraphNotice, GraphNoticeWords, GraphOverlay, GraphOverlayResponse, NoticeTone, OverlayTarget,
    place_overlay,
};
pub use picker::{
    BlockPalette, BlockPaletteResponse, BlockPaletteWords, BlockPicker, BlockPickerResponse, BlockPickerWords,
};
pub use pipeline::{PipelineGraph, PipelineStep, step_links};
pub use run::{RunGraph, RunStep, format_duration, run_link};
pub use toolbar::{GraphToolbar, GraphToolbarResponse, GraphToolbarWords};
pub use view_buttons::{ButtonsOrientation, GraphControlButton, GraphViewButtons, GraphViewWords, ViewButtonsResponse};
