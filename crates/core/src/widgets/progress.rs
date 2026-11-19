//! How far something has come, after sokkel-ui's `Progress` primitive, and a health report, after
//! its `CheckList`.

use egui::{Color32, CornerRadius, Response, Sense, Stroke, TextWrapMode, Ui, Widget, WidgetText, pos2, vec2};

use super::download_button::format_percent;
use crate::{Icon, TextRole, icons, tokens};

/// A bar of how far something has come, after sokkel-ui's `Progress` with its `ProgressLabel` and
/// `ProgressValue`: the label at the start and the value at the end on one line, the track (4 tall,
/// `muted`, round) across the width 12 under them with the share in `primary`. The value is said
/// as `format_percent` writes a share unless the app writes it.
#[must_use = "add it with `ui.add(...)`"]
pub struct Progress {
    /// From 0 to 1.
    share: f32,
    label: Option<String>,
    /// The label at the body's weight in the muted ink, as `font-normal text-muted-foreground`.
    quiet: bool,
    /// The label and the value in the small words, as `text-xs`.
    small: bool,
    value: Option<String>,
    show_value: bool,
}

impl Progress {
    /// `value` from 0 to 100, as the web's `value` with its `max` of 100.
    pub fn new(value: f32) -> Self {
        Self {
            share: if value.is_finite() {
                (value / 100.0).clamp(0.0, 1.0)
            } else {
                0.0
            },
            label: None,
            quiet: false,
            small: false,
            value: None,
            show_value: true,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// The label at the body's weight in the muted ink, as a label of facts rather than a title.
    pub fn quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    /// The label and the value in the small words, as a progress in a table's cell.
    pub fn small(mut self, small: bool) -> Self {
        self.small = small;
        self
    }

    /// The value written by the app, in place of the share in percent.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// No value at the end, the bar alone with its label.
    pub fn no_value(mut self) -> Self {
        self.show_value = false;
        self
    }
}

impl Widget for Progress {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let width = ui.available_width();
        let role = if self.small { TextRole::Help } else { TextRole::Body };
        let label_role = if self.small || self.quiet {
            role
        } else {
            TextRole::Label
        };
        let label_ink = if self.quiet || self.small {
            t.color("muted-foreground")
        } else {
            t.color("foreground")
        };
        let said = self.value.clone().unwrap_or_else(|| format_percent(self.share));
        let value = self.show_value.then(|| {
            WidgetText::from(role.rich(&t, said).color(t.color("muted-foreground"))).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                role.font(&t),
            )
        });
        let room = width - value.as_ref().map_or(0.0, |one| one.size().x + 12.0);
        let label = self.label.as_ref().map(|label| {
            WidgetText::from(label_role.rich(&t, label).color(label_ink)).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                room.max(0.0),
                label_role.font(&t),
            )
        });
        let line = if label.is_some() || value.is_some() {
            role.line_height(&t)
        } else {
            0.0
        };
        let gap = if line > 0.0 { 12.0 } else { 0.0 };
        let track = 4.0;
        let (rect, response) = ui.allocate_exact_size(vec2(width, line + gap + track), Sense::hover());
        let painter = ui.painter();
        if let Some(label) = label {
            let y = rect.top() + (line - label.size().y) / 2.0;
            painter.galley(pos2(rect.left(), y), label, label_ink);
        }
        if let Some(value) = value {
            let y = rect.top() + (line - value.size().y) / 2.0;
            painter.galley(
                pos2(rect.right() - value.size().x, y),
                value,
                t.color("muted-foreground"),
            );
        }
        let bar = egui::Rect::from_min_size(pos2(rect.left(), rect.bottom() - track), vec2(width, track));
        let round = CornerRadius::same((track / 2.0) as u8);
        painter.rect_filled(bar, round, t.color("muted"));
        if self.share > 0.0 {
            let filled = egui::Rect::from_min_size(bar.min, vec2(width * self.share, track));
            painter.rect_filled(filled, round, t.color("primary"));
        }
        response.on_hover_text_at_pointer(format_percent(self.share))
    }
}

/// What a check found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum CheckStatus {
    #[default]
    Ok,
    Warn,
    Fail,
}

impl CheckStatus {
    fn color(self) -> &'static str {
        match self {
            CheckStatus::Ok => "good",
            CheckStatus::Warn => "warning",
            CheckStatus::Fail => "critical",
        }
    }
}

/// One health check: what was checked, what it found, and what to do when it is not ok; `group`
/// names the check it nests under.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Check {
    pub name: String,
    pub status: CheckStatus,
    pub detail: Option<String>,
    pub hint: Option<String>,
    pub group: Option<String>,
}

impl Check {
    pub fn new(name: impl Into<String>, status: CheckStatus) -> Self {
        Self {
            name: name.into(),
            status,
            ..Self::default()
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn hint(mut self, hint: Option<impl Into<String>>) -> Self {
        self.hint = hint.map(Into::into);
        self
    }

    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }
}

/// The worst status among some checks, `Ok` for none, as the web's `worstStatus`.
pub fn worst_status(checks: &[&Check]) -> CheckStatus {
    checks.iter().map(|one| one.status).max().unwrap_or_default()
}

/// The checks as a two-level tree by each check's `group`, as the web's `checkTree`: a check nests
/// under the top-level check its groups lead up to; one naming a group not in the list, or whose
/// groups run in a circle, stays at the top level, and the order is the order given.
pub fn check_tree(checks: &[Check]) -> Vec<(&Check, Vec<&Check>)> {
    let parent = |check: &Check| -> Option<&Check> {
        let group = check.group.as_deref()?;
        if group == check.name {
            return None;
        }
        checks.iter().find(|one| one.name == group)
    };
    let top = |check: &Check| -> Option<&Check> {
        let mut seen = vec![check.name.as_str()];
        let mut up = parent(check);
        while let Some(at) = up {
            if seen.contains(&at.name.as_str()) {
                return None;
            }
            seen.push(at.name.as_str());
            match parent(at) {
                Some(next) => up = Some(next),
                None => return Some(at),
            }
        }
        None
    };
    let homes: Vec<Option<&Check>> = checks.iter().map(top).collect();
    let mut nodes: Vec<(&Check, Vec<&Check>)> = checks
        .iter()
        .zip(&homes)
        .filter(|(_, home)| home.is_none())
        .map(|(check, _)| (check, Vec::new()))
        .collect();
    for (check, home) in checks.iter().zip(&homes) {
        if let Some(home) = home
            && let Some(node) = nodes.iter_mut().find(|(one, _)| std::ptr::eq(*one, *home))
        {
            node.1.push(check);
        }
    }
    nodes
}

/// A health report, after sokkel-ui's `CheckList`: each check with its verdict as an 8 dot (`good`,
/// `warning`, `critical`), its name, what it found under it in the small muted words and a hint in
/// `warning-ink` when it needs attention, 12 across and 8 over and under. A check with checks under
/// it wears the worst of them and opens to show them under a rule; one that is not ok starts open.
pub struct CheckList<'a> {
    id: egui::Id,
    checks: &'a [Check],
}

impl<'a> CheckList<'a> {
    pub fn new(id: impl egui::AsIdSalt, checks: &'a [Check]) -> Self {
        Self {
            id: egui::Id::unique(id),
            checks,
        }
    }

    pub fn show(self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (check, children) in check_tree(self.checks) {
                if children.is_empty() {
                    line(ui, check, check.status, 12.0, 8.0, None);
                    continue;
                }
                let mut all = vec![check];
                all.extend(children.iter().copied());
                let status = worst_status(&all);
                let key = self.id.with(&check.name);
                let mut open = ui
                    .ctx()
                    .data(|d| d.get_temp::<bool>(key))
                    .unwrap_or(status != CheckStatus::Ok);
                let response = line(ui, check, status, 12.0, 8.0, Some(open));
                if response.clicked() {
                    open = !open;
                }
                ui.ctx().data_mut(|d| d.insert_temp(key, open));
                if open {
                    let t = tokens(ui.ctx());
                    let top = ui.cursor().top();
                    ui.horizontal(|ui| {
                        ui.add_space(17.0);
                        ui.vertical(|ui| {
                            for child in &children {
                                line(ui, child, child.status, 12.0, 6.0, None);
                            }
                        });
                    });
                    let bottom = ui.min_rect().bottom();
                    let x = ui.min_rect().left() + 17.0;
                    ui.painter().vline(x, top..=bottom, Stroke::new(1.0, t.color("border")));
                }
            }
        });
    }
}

/// One check's line: its dot, its name with what it found and its hint under it, and a chevron
/// when it opens; pressed, when it does.
fn line(ui: &mut Ui, check: &Check, status: CheckStatus, across: f32, over: f32, open: Option<bool>) -> Response {
    let t = tokens(ui.ctx());
    let width = ui.available_width();
    let chevron = if open.is_some() { 16.0 + 10.0 } else { 0.0 };
    let words = (width - across * 2.0 - 8.0 - 10.0 - chevron).max(0.0);
    let galley = |text: &str, role: TextRole, ink: Color32| {
        WidgetText::from(role.rich(&t, text).color(ink)).into_galley(ui, Some(TextWrapMode::Wrap), words, role.font(&t))
    };
    let name = galley(&check.name, TextRole::Body, t.color("foreground"));
    let detail = check
        .detail
        .as_deref()
        .map(|one| galley(one, TextRole::Help, t.color("muted-foreground")));
    let hint = check
        .hint
        .as_deref()
        .map(|one| galley(one, TextRole::Help, t.color("warning-ink")));
    let height =
        name.size().y + detail.as_ref().map_or(0.0, |one| one.size().y) + hint.as_ref().map_or(0.0, |one| one.size().y);
    let sense = if open.is_some() { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(vec2(width, height + over * 2.0), sense);
    let painter = ui.painter();
    if open.is_some() && response.hovered() {
        painter.rect_filled(rect, CornerRadius::ZERO, t.color("hover"));
    }
    let left = rect.left() + across;
    let mut y = rect.top() + over;
    painter.circle_filled(pos2(left + 4.0, y + 6.0 + 4.0), 4.0, t.color(status.color()));
    let x = left + 8.0 + 10.0;
    let name_height = name.size().y;
    painter.galley(pos2(x, y), name, t.color("foreground"));
    y += name_height;
    for one in [detail, hint].into_iter().flatten() {
        let tall = one.size().y;
        painter.galley(pos2(x, y), one, t.color("muted-foreground"));
        y += tall;
    }
    if let Some(open) = open {
        let glyph = if open { Icon::ChevronDown } else { Icon::ChevronRight };
        icons::paint(
            painter,
            pos2(rect.right() - across - 8.0, rect.top() + over + 2.0 + 8.0),
            glyph,
            16.0,
            t.color("muted-foreground"),
        );
    }
    response
}
