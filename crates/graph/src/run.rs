//! A run of a pipeline as a graph, after sokkel-ui-graph's `RunGraph`: each step with its state,
//! its duration and its progress, and edges that carry where the run went.

use std::collections::{HashMap, HashSet};

use egui::{Color32, Ui, Vec2};
use sokkel_egui::{Icon, Tokens, tokens};

use crate::canvas::{
    EdgeMotion, EdgeTone, GraphCanvas, GraphFrame, GraphLink, GraphResponse, GraphWords, fit_direction, mix,
};
use crate::cycles::StepNeeds;
use crate::layout::{GraphDirection, GraphSpacing};
use crate::node::{GraphNode, NodeIssue};

/// The states a run has not reached yet, whose edges are drawn quiet.
const AHEAD: [&str; 5] = ["queued", "pending", "waiting", "skipped", "cancelled"];
/// The states in which a step has an output to hand on.
const PRODUCED: [&str; 2] = ["succeeded", "completed_with_errors"];
/// The states in which a step can still read what an edge brings it.
const READING: [&str; 4] = ["pending", "queued", "waiting", "running"];
/// How long a handover's pulse is held on its edges, a little past its animation.
const PULSE_SECONDS: f64 = 0.7;

/// One step of a run: the pipeline's step, with how far it got.
#[derive(Clone, Debug, Default)]
pub struct RunStep {
    pub code: String,
    pub name: Option<String>,
    pub kind: Option<String>,
    pub needs: Vec<String>,
    pub icon: Option<Icon>,
    pub issues: Vec<NodeIssue>,
    /// A run state: `succeeded`, `failed`, `running`, `queued`, `pending`, `waiting`, `cancelled`,
    /// `skipped` or `completed_with_errors`.
    pub status: String,
    pub duration_ms: Option<f64>,
    /// How far the step has come, from 0 to 1, drawn along its foot in its state's color.
    pub progress: Option<f32>,
}

impl RunStep {
    pub fn new(code: impl Into<String>, status: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            status: status.into(),
            ..Self::default()
        }
    }

    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
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

    pub fn duration_ms(mut self, ms: Option<f64>) -> Self {
        self.duration_ms = ms;
        self
    }

    pub fn progress(mut self, progress: Option<f32>) -> Self {
        self.progress = progress;
        self
    }
}

impl StepNeeds for RunStep {
    fn code(&self) -> &str {
        &self.code
    }

    fn needs(&self) -> &[String] {
        &self.needs
    }
}

/// A duration in milliseconds, said briefly in English.
pub fn format_duration(ms: f64) -> String {
    if ms < 1000.0 {
        return format!("{} ms", ms.round());
    }
    let seconds = ms / 1000.0;
    if seconds < 60.0 {
        return if seconds < 10.0 {
            format!("{seconds:.1} s")
        } else {
            format!("{seconds:.0} s")
        };
    }
    let minutes = (seconds / 60.0).floor();
    if minutes < 60.0 {
        return format!("{minutes}m {}s", (seconds % 60.0).round());
    }
    format!("{}h {}m", (minutes / 60.0).floor(), minutes % 60.0)
}

/// How an edge of a run is drawn, from the states of its two ends: quiet toward a step not reached
/// and strong into one running; the state its source settled in as its hue; dashed into a step
/// that was skipped; and moving while a step that has produced its output feeds one still running.
/// A source that has just produced plays one pulse into each step that can still read it. With
/// motion reduced, a moving edge stands lit and the pulse is left out.
pub fn run_link(
    source: Option<&str>,
    target: Option<&str>,
    handing: bool,
    reduced: bool,
) -> (EdgeTone, Option<String>, bool, Option<EdgeMotion>) {
    let tone = match target {
        None => EdgeTone::Plain,
        Some("running") => EdgeTone::Strong,
        Some(state) if AHEAD.contains(&state) => EdgeTone::Quiet,
        Some(_) => EdgeTone::Plain,
    };
    let tint = source
        .filter(|s| !AHEAD.contains(s) && *s != "running")
        .map(str::to_owned);
    let produced = source.is_some_and(|s| PRODUCED.contains(&s));
    let motion = match target {
        Some(target) if produced && READING.contains(&target) => {
            if handing && !reduced {
                Some(EdgeMotion::Pulse)
            } else if target != "running" {
                None
            } else if reduced {
                Some(EdgeMotion::Lit)
            } else {
                Some(EdgeMotion::Flowing)
            }
        }
        _ => None,
    };
    (tone, tint, target == Some("skipped"), motion)
}

/// A state's token: `status-completed-with-errors` for `completed_with_errors`.
fn status_key(status: &str) -> String {
    format!("status-{}", status.replace('_', "-"))
}

/// A state's color, or the muted ink for a state the tokens do not name.
fn status_color(t: &Tokens, status: &str) -> Color32 {
    t.color_or(&status_key(status), t.color("muted-foreground"))
}

/// A run step's box: its state's dot and edge, its duration, and its progress.
fn run_node(t: &Tokens, step: &RunStep) -> GraphNode {
    let color = status_color(t, &step.status);
    let ink = |state: &str| t.color_or(&format!("{}-ink", status_key(state)), color);
    let mut node = GraphNode::new(&step.code)
        .status(step.status.replace('_', " "), color)
        .duration(step.duration_ms.map(format_duration))
        .progress(step.progress)
        .issues(step.issues.clone());
    if let Some(name) = &step.name {
        node = node.name(name);
    }
    if let Some(kind) = &step.kind {
        node = node.kind(kind);
    }
    if let Some(icon) = step.icon {
        node = node.icon(icon);
    }
    match step.status.as_str() {
        "succeeded" | "completed_with_errors" => node.border(Some(ink(&step.status)), 1.0),
        "failed" => node
            .border(Some(ink("failed")), 2.0)
            .fill(Some(mix(color, t.color("node"), 0.12))),
        "running" | "waiting" => node.border(Some(color), 1.0),
        "skipped" => node.source(true).muted(true),
        "cancelled" => node.muted(true),
        _ => node,
    }
}

/// What a run graph keeps between frames: the states it saw, and the steps handing over now.
#[derive(Clone, Debug, Default)]
struct Seen {
    states: Option<HashMap<String, String>>,
    handing: HashSet<String>,
    until: f64,
}

/// A run of a pipeline as a graph: each step with its status, its duration and its progress, and
/// edges that carry where the run went (see `run_link`). Boxes keep room for a duration, so a step
/// changing state moves no node. It selects only, on a stage, down below 560 wide and right above.
pub struct RunGraph<'a> {
    id: egui::Id,
    steps: &'a [RunStep],
    selected: Option<&'a str>,
    direction: Option<GraphDirection>,
    frame: GraphFrame,
    spacing: GraphSpacing,
    controls: bool,
    size: Option<Vec2>,
    words: GraphWords,
}

impl<'a> RunGraph<'a> {
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug, steps: &'a [RunStep]) -> Self {
        Self {
            id: egui::Id::unique(("sokkel-run-graph", id)),
            steps,
            selected: None,
            direction: None,
            frame: GraphFrame::Stage,
            spacing: GraphSpacing::NORMAL,
            controls: true,
            size: None,
            words: GraphWords::default(),
        }
    }

    pub fn selected(mut self, selected: Option<&'a str>) -> Self {
        self.selected = selected;
        self
    }

    pub fn direction(mut self, direction: Option<GraphDirection>) -> Self {
        self.direction = direction;
        self
    }

    pub fn frame(mut self, frame: GraphFrame) -> Self {
        self.frame = frame;
        self
    }

    pub fn spacing(mut self, spacing: GraphSpacing) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn controls(mut self, controls: bool) -> Self {
        self.controls = controls;
        self
    }

    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }

    pub fn words(mut self, words: GraphWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> GraphResponse {
        let t = tokens(ui.ctx());
        let now = ui.input(|i| i.time);
        let reduced = ui.style().animation_time <= 0.0;
        let width = self.size.map_or_else(|| ui.available_width(), |s| s.x);
        let direction = self.direction.unwrap_or_else(|| fit_direction(width));

        // the steps that have just produced their output: a step the graph had not seen has handed
        // nothing over, so opening a finished run pulses nothing
        let seen_id = self.id.with("seen");
        let mut seen: Seen = ui.data(|d| d.get_temp(seen_id)).unwrap_or_default();
        let states: HashMap<String, String> = self.steps.iter().map(|s| (s.code.clone(), s.status.clone())).collect();
        if let Some(before) = &seen.states {
            let now_handing: HashSet<String> = self
                .steps
                .iter()
                .filter(|s| {
                    before
                        .get(&s.code)
                        .is_some_and(|was| !PRODUCED.contains(&was.as_str()) && PRODUCED.contains(&s.status.as_str()))
                })
                .map(|s| s.code.clone())
                .collect();
            if !now_handing.is_empty() {
                seen.handing = now_handing;
                seen.until = now + PULSE_SECONDS;
            }
        }
        if now >= seen.until {
            seen.handing.clear();
        } else {
            ui.ctx().request_repaint();
        }
        seen.states = Some(states.clone());
        ui.data_mut(|d| d.insert_temp(seen_id, seen.clone()));

        let nodes: Vec<GraphNode> = self.steps.iter().map(|s| run_node(&t, s)).collect();
        let links: Vec<GraphLink> = self
            .steps
            .iter()
            .flat_map(|step| {
                step.needs.iter().map(|need| {
                    let (tone, tint, dashed, motion) = run_link(
                        states.get(need).map(String::as_str),
                        Some(step.status.as_str()),
                        seen.handing.contains(need),
                        reduced,
                    );
                    GraphLink::new(need, &step.code)
                        .tone(tone)
                        .tint(tint.map(|state| status_color(&t, &state)))
                        .dashed(dashed)
                        .motion(motion)
                })
            })
            .collect();
        let mut canvas = GraphCanvas::new(self.id, &nodes, &links)
            .frame(self.frame)
            .direction(direction)
            .spacing(self.spacing)
            .selected(self.selected)
            .controls(self.controls)
            .words(self.words);
        if let Some(size) = self.size {
            canvas = canvas.size(size);
        }
        canvas.show_response(ui)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_read_briefly() {
        assert_eq!(format_duration(420.0), "420 ms");
        assert_eq!(format_duration(4200.0), "4.2 s");
        assert_eq!(format_duration(42_000.0), "42 s");
        assert_eq!(format_duration(125_000.0), "2m 5s");
        assert_eq!(format_duration(7_500_000.0), "2h 5m");
    }

    #[test]
    fn an_edge_carries_where_the_run_went() {
        // a step that produced feeds one running: the edge flows, darker, in the source's hue
        let (tone, tint, dashed, motion) = run_link(Some("succeeded"), Some("running"), false, false);
        assert_eq!(tone, EdgeTone::Strong);
        assert_eq!(tint.as_deref(), Some("succeeded"));
        assert!(!dashed);
        assert_eq!(motion, Some(EdgeMotion::Flowing));
        // with motion reduced it stands lit, and a handover pulses only with motion
        assert_eq!(
            run_link(Some("succeeded"), Some("running"), false, true).3,
            Some(EdgeMotion::Lit)
        );
        assert_eq!(
            run_link(Some("succeeded"), Some("queued"), true, false).3,
            Some(EdgeMotion::Pulse)
        );
        // into a step not reached it is quiet, and into one skipped dashed
        let (tone, tint, dashed, _) = run_link(Some("running"), Some("skipped"), false, false);
        assert_eq!(tone, EdgeTone::Quiet);
        assert_eq!(tint, None);
        assert!(dashed);
    }
}
