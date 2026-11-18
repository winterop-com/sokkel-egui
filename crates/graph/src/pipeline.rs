//! A pipeline's steps as a graph, after sokkel-ui-graph's `PipelineGraph`.

use std::collections::HashMap;

use egui::{Pos2, Ui, Vec2};
use sokkel_egui::Icon;

use crate::canvas::{
    GraphCanvas, GraphEditing, GraphFrame, GraphLink, GraphMenu, GraphResponse, GraphWords, NodeMenuFn, PaneMenuFn,
    fit_direction,
};
use crate::cycles::StepNeeds;
use crate::layout::{GraphDirection, GraphSpacing};
use crate::node::{GraphNode, NodeIssue};
use crate::toolbar::GraphToolbar;
use crate::view_buttons::GraphControlButton;

/// One step of a pipeline: its code, an optional name and kind, the codes of the steps it needs, a
/// glyph such as its block's, and what is wrong with it.
#[derive(Clone, Debug, Default)]
pub struct PipelineStep {
    pub code: String,
    pub name: Option<String>,
    pub kind: Option<String>,
    pub needs: Vec<String>,
    pub icon: Option<Icon>,
    pub issues: Vec<NodeIssue>,
}

impl PipelineStep {
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            ..Self::default()
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    pub fn needs(mut self, needs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.needs = needs.into_iter().map(Into::into).collect();
        self
    }

    pub fn icon(mut self, icon: Option<Icon>) -> Self {
        self.icon = icon;
        self
    }

    pub fn issues(mut self, issues: Vec<NodeIssue>) -> Self {
        self.issues = issues;
        self
    }

    /// The step's box.
    pub fn node(&self) -> GraphNode {
        let mut node = GraphNode::new(&self.code).issues(self.issues.clone());
        if let Some(name) = &self.name {
            node = node.name(name);
        }
        if let Some(kind) = &self.kind {
            node = node.kind(kind);
        }
        if let Some(icon) = self.icon {
            node = node.icon(icon);
        }
        node
    }
}

impl StepNeeds for PipelineStep {
    fn code(&self) -> &str {
        &self.code
    }

    fn needs(&self) -> &[String] {
        &self.needs
    }
}

/// The edges of a set of steps: one from every step a step needs, to it.
pub fn step_links<S: StepNeeds>(steps: &[S]) -> Vec<GraphLink> {
    steps
        .iter()
        .flat_map(|step| step.needs().iter().map(|need| GraphLink::new(need, step.code())))
        .collect()
}

/// A pipeline's steps as a graph, each step after the steps it needs, on a stage: down below 560
/// wide and right above, unless told. It selects only, unless given `editing`; everything a
/// `GraphCanvas` takes on a stage it takes too, and it answers as the canvas does.
pub struct PipelineGraph<'a> {
    id: egui::Id,
    steps: &'a [PipelineStep],
    selected: Option<&'a str>,
    direction: Option<GraphDirection>,
    frame: GraphFrame,
    editing: Option<GraphEditing<'a>>,
    positions: Option<&'a HashMap<String, Pos2>>,
    controls: bool,
    extra_controls: &'a [GraphControlButton],
    toolbar: Option<GraphToolbar>,
    empty_action: Option<(Option<Icon>, String)>,
    node_menu: Option<NodeMenuFn<'a>>,
    pane_menu: Option<PaneMenuFn<'a>>,
    spacing: GraphSpacing,
    size: Option<Vec2>,
    words: GraphWords,
}

impl<'a> PipelineGraph<'a> {
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug, steps: &'a [PipelineStep]) -> Self {
        Self {
            id: egui::Id::unique(("sokkel-pipeline-graph", id)),
            steps,
            selected: None,
            direction: None,
            frame: GraphFrame::Stage,
            editing: None,
            positions: None,
            controls: true,
            extra_controls: &[],
            toolbar: None,
            empty_action: None,
            node_menu: None,
            pane_menu: None,
            spacing: GraphSpacing::NORMAL,
            size: None,
            words: GraphWords::default(),
        }
    }

    pub fn selected(mut self, selected: Option<&'a str>) -> Self {
        self.selected = selected;
        self
    }

    /// The way the graph runs at every width; left out, down below 560 wide and right above.
    pub fn direction(mut self, direction: Option<GraphDirection>) -> Self {
        self.direction = direction;
        self
    }

    pub fn frame(mut self, frame: GraphFrame) -> Self {
        self.frame = frame;
        self
    }

    /// Makes the graph an editor; each gesture is reported and the steps change only when the app
    /// changes them.
    pub fn editing(mut self, editing: Option<GraphEditing<'a>>) -> Self {
        self.editing = editing;
        self
    }

    /// Where steps were moved to, by code, over where the layout put them.
    pub fn positions(mut self, positions: Option<&'a HashMap<String, Pos2>>) -> Self {
        self.positions = positions;
        self
    }

    /// False hides the zoom buttons, for a toolbar that carries them.
    pub fn controls(mut self, controls: bool) -> Self {
        self.controls = controls;
        self
    }

    /// The app's own buttons after the zoom buttons, such as one that adds a step.
    pub fn extra_controls(mut self, extra: &'a [GraphControlButton]) -> Self {
        self.extra_controls = extra;
        self
    }

    /// A bar over the canvas's top-left corner.
    pub fn toolbar(mut self, toolbar: Option<GraphToolbar>) -> Self {
        self.toolbar = toolbar;
        self
    }

    /// What the canvas offers while it has no steps, such as a button that adds the first.
    pub fn empty_action(mut self, label: impl Into<String>, icon: Option<Icon>) -> Self {
        self.empty_action = Some((icon, label.into()));
        self
    }

    pub fn node_menu(mut self, menu: &'a dyn Fn(&str) -> Option<GraphMenu>) -> Self {
        self.node_menu = Some(menu);
        self
    }

    pub fn pane_menu(mut self, menu: &'a dyn Fn(Pos2) -> Option<GraphMenu>) -> Self {
        self.pane_menu = Some(menu);
        self
    }

    pub fn spacing(mut self, spacing: GraphSpacing) -> Self {
        self.spacing = spacing;
        self
    }

    /// The canvas's size; the room it is given when left out.
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }

    pub fn words(mut self, words: GraphWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> GraphResponse {
        let width = self.size.map_or_else(|| ui.available_width(), |s| s.x);
        let direction = self.direction.unwrap_or_else(|| fit_direction(width));
        let nodes: Vec<GraphNode> = self.steps.iter().map(PipelineStep::node).collect();
        let links = step_links(self.steps);
        let mut canvas = GraphCanvas::new(self.id, &nodes, &links)
            .frame(self.frame)
            .direction(direction)
            .spacing(self.spacing)
            .selected(self.selected)
            .editing(self.editing)
            .positions(self.positions)
            .controls(self.controls)
            .extra_controls(self.extra_controls)
            .toolbar(self.toolbar)
            .words(self.words);
        if let Some(size) = self.size {
            canvas = canvas.size(size);
        }
        if let Some((icon, label)) = self.empty_action {
            canvas = canvas.empty_action(label, icon);
        }
        if let Some(menu) = self.node_menu {
            canvas = canvas.node_menu(menu);
        }
        if let Some(menu) = self.pane_menu {
            canvas = canvas.pane_menu(menu);
        }
        canvas.show_response(ui)
    }
}
