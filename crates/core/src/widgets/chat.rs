//! A conversation with an assistant, after sokkel-ui's chat parts: the usage line under an answer
//! (`UsageLine`), the model's thinking folded behind one line, the assistant's reply in a card
//! (`AssistantReply`), the person's question in a bubble with its reactions (`Bubble`,
//! `ReactionChips`), one model's answer beside others' (`AnswerColumn`), the column a conversation
//! is laid out in (`Conversation`), the box a message is written in (`Composer`, after
//! sokkel-design's `composer` spec), and the conversations beside a chat (`ConversationHistory`).

use egui::{
    Align, Color32, CornerRadius, Frame, Id, Key, Layout, Margin, Modifiers, Rect, Response, Sense, Stroke, StrokeKind,
    TextEdit, TextWrapMode, Ui, WidgetText, pos2, text::LayoutJob, vec2,
};

use super::{DISABLED, dim, spinner};
use crate::reactions::Reaction;
use crate::{
    Button, ButtonSize, ButtonVariant, Cell, Chip, ChipTone, Column, ConfirmAnswer, ConfirmDialog, DataTable,
    DropdownMenu, Editor, Icon, IconButton, Markdown, MarkdownWords, MenuItem, Problem, QUICK_REACTIONS, TableState,
    TableVariant, TableWords, TextRole, Tokens, icons, mono_font, mono_size, tokens,
};

// --- Usage line ------------------------------------------------------------------------------------

/// The words a usage line says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageLineWords {
    /// After the tokens the prompt took: `412 in`.
    pub input: String,
    /// After the tokens written: `87 out`.
    pub output: String,
    /// After the tokens written while the answer is still coming: `87 out so far`.
    pub so_far: String,
    /// Before the time to the first token: `first token 240 ms`.
    pub first_token: String,
    /// After the rate: `31.4 tok/s`.
    pub rate: String,
}

impl Default for UsageLineWords {
    fn default() -> Self {
        Self {
            input: "in".into(),
            output: "out".into(),
            so_far: "so far".into(),
            first_token: "first token".into(),
            rate: "tok/s".into(),
        }
    }
}

/// A time in milliseconds as a usage line writes it: under a second as it is, then seconds, two
/// decimals under ten and one above: `92 ms`, `5.09 s`, `12.4 s`.
pub fn usage_time(ms: f64) -> String {
    if ms < 1000.0 {
        return format!("{} ms", js_round(ms));
    }
    let seconds = ms / 1000.0;
    format!("{} s", to_fixed(seconds, if seconds < 10.0 { 2 } else { 1 }))
}

/// A rate in tokens a second: a hundred or more whole, a slower one with a decimal.
pub fn usage_speed(per_second: f64) -> String {
    if per_second >= 100.0 {
        js_round(per_second).to_string()
    } else {
        to_fixed(per_second, 1)
    }
}

/// A count of tokens with its thousands grouped: `1,284`.
pub fn usage_count(tokens: f64) -> String {
    let whole = js_round(tokens);
    let digits = whole.unsigned_abs().to_string();
    let mut out = String::new();
    for (at, c) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if whole < 0 { format!("-{out}") } else { out }
}

/// `Math.round`: halves up.
fn js_round(value: f64) -> i64 {
    (value + 0.5).floor() as i64
}

/// `Number.toFixed`.
fn to_fixed(value: f64, digits: usize) -> String {
    format!("{value:.digits$}")
}

/// What an answer cost and how fast it came, in one quiet line under it, as sokkel-ui's
/// `UsageLine`: the tokens in and out, the time to the first token, the rate and how long it took,
/// in the code face at the help size, the figures in the text's ink and their units muted, 12
/// apart. A figure not given is left out; `live` says the count is so far.
#[must_use = "add it with `ui.add(...)`"]
#[derive(Clone, Debug, Default)]
pub struct UsageLine {
    input: Option<f64>,
    output: f64,
    first_token: Option<f64>,
    rate: Option<f64>,
    latency: Option<f64>,
    note: Option<String>,
    live: bool,
    words: UsageLineWords,
}

impl UsageLine {
    /// The line for an answer of `output` tokens.
    pub fn new(output: f64) -> Self {
        Self {
            output,
            ..Self::default()
        }
    }

    /// The tokens the prompt took.
    pub fn input(mut self, tokens: f64) -> Self {
        self.input = Some(tokens);
        self
    }

    /// Milliseconds from the request to the first token.
    pub fn first_token(mut self, ms: f64) -> Self {
        self.first_token = Some(ms);
        self
    }

    /// Tokens written a second, from the first token.
    pub fn rate(mut self, per_second: f64) -> Self {
        self.rate = Some(per_second);
        self
    }

    /// Milliseconds the whole answer took, or has taken so far.
    pub fn latency(mut self, ms: f64) -> Self {
        self.latency = Some(ms);
        self
    }

    /// A last quiet fact, such as why the answer ended: `max tokens`.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// The answer is still coming: the count says so far.
    pub fn live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }

    pub fn words(mut self, words: UsageLineWords) -> Self {
        self.words = words;
        self
    }

    /// The line's pieces, each a run of figures and words: whether each part is a figure.
    fn pieces(&self) -> Vec<Vec<(String, bool)>> {
        let w = &self.words;
        let mut out = Vec::new();
        if let Some(input) = self.input {
            out.push(vec![(usage_count(input), true), (format!(" {}", w.input), false)]);
        }
        let mut output = vec![(usage_count(self.output), true), (format!(" {}", w.output), false)];
        if self.live {
            output.push((format!(" {}", w.so_far), false));
        }
        out.push(output);
        if let Some(first) = self.first_token {
            out.push(vec![(format!("{} ", w.first_token), false), (usage_time(first), true)]);
        }
        if let Some(rate) = self.rate {
            out.push(vec![(usage_speed(rate), true), (format!(" {}", w.rate), false)]);
        }
        if let Some(latency) = self.latency {
            out.push(vec![(usage_time(latency), true)]);
        }
        if let Some(note) = &self.note {
            out.push(vec![(note.clone(), false)]);
        }
        out
    }
}

impl egui::Widget for UsageLine {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let help = TextRole::Help.font(&t).size;
        let font = mono_font(ui.ctx(), mono_size(&t, help), 400.0);
        let line = TextRole::Help.line_height(&t);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(12.0, 2.0);
            for piece in self.pieces() {
                let mut job = LayoutJob::default();
                for (text, figure) in piece {
                    job.append(
                        &text,
                        0.0,
                        egui::TextFormat {
                            font_id: font.clone(),
                            color: if figure {
                                t.color("foreground")
                            } else {
                                t.color("muted-foreground")
                            },
                            line_height: Some(line),
                            ..Default::default()
                        },
                    );
                }
                ui.label(job);
            }
        })
        .response
    }
}

// --- Thinking --------------------------------------------------------------------------------------

/// How many words a text holds, markdown marks and all: the runs between spaces holding a letter
/// or a digit.
pub fn word_count(text: &str) -> usize {
    text.split_whitespace()
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .count()
}

/// The model's thinking, folded behind one line that counts its words: open while it is still
/// written (`thinking`), the line saying `thinking_word`; once the answer begins it folds, and a
/// press on the line opens it.
fn reasoning(ui: &mut Ui, id: Id, text: &str, thinking: bool, words: (&str, &str), markdown: &MarkdownWords) {
    let t = tokens(ui.ctx());
    let open_id = id.with("thought-open");
    let mut open = ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
    let said = if thinking {
        words.0.to_owned()
    } else {
        words
            .1
            .replace("{count}", &word_count(text).to_string())
            .replace("{words}", if word_count(text) == 1 { "word" } else { "words" })
    };
    let line = trace_line(ui, &t, Icon::Brain, &said, !thinking, open);
    if line.clicked() && !thinking {
        open = !open;
        ui.data_mut(|d| d.insert_temp(open_id, open));
    }
    if thinking || open {
        ui.scope(|ui| {
            ui.visuals_mut().override_text_color = Some(t.color("muted-foreground"));
            Markdown::new(text)
                .inline(TextRole::Body, "muted-foreground")
                .live(!thinking)
                .words(markdown.clone())
                .show(ui);
        });
    }
}

/// One line of a trace: an icon 16, its words in the body size, muted and the foreground under the
/// pointer, and a chevron after them when it folds something.
fn trace_line(ui: &mut Ui, t: &Tokens, icon: Icon, said: &str, folds: bool, open: bool) -> Response {
    let role = TextRole::Body;
    let galley = WidgetText::from(role.rich(t, said)).into_galley(
        ui,
        Some(TextWrapMode::Truncate),
        (ui.available_width() - 48.0).max(1.0),
        role.font(t),
    );
    let chevron = if folds { 20.0 } else { 0.0 };
    let height = role.line_height(t);
    let size = vec2(16.0 + 8.0 + galley.size().x + chevron, height);
    let (rect, response) = ui.allocate_exact_size(size, if folds { Sense::click() } else { Sense::hover() });
    let ink = if response.hovered() && folds {
        t.color("foreground")
    } else {
        t.color("muted-foreground")
    };
    icons::paint(ui.painter(), pos2(rect.left() + 8.0, rect.center().y), icon, 16.0, ink);
    let text_at = pos2(rect.left() + 24.0, rect.center().y - galley.size().y / 2.0);
    let width = galley.size().x;
    ui.painter().galley(text_at, galley, ink);
    if folds {
        icons::paint(
            ui.painter(),
            pos2(text_at.x + width + 10.0, rect.center().y),
            if open { Icon::ChevronDown } else { Icon::ChevronRight },
            16.0,
            ink,
        );
    }
    response
}

// --- Assistant reply -------------------------------------------------------------------------------

/// Where an answer is: written, still being written, stopped by the person, or failed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AssistantState {
    #[default]
    Done,
    Streaming,
    Stopped,
    Failed,
}

/// The feedback given to an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplyFeedback {
    Good,
    Bad,
}

/// The words an assistant's reply says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssistantReplyWords {
    /// The speaker when no name is given.
    pub name: String,
    pub copy: String,
    pub copied: String,
    /// The button that asks again, in the foot and on a failed answer.
    pub try_again: String,
    pub good: String,
    pub bad: String,
    pub previous: String,
    pub next: String,
    /// `{at} of {of}`.
    pub version: String,
    pub stopped: String,
    /// The title of a failed answer, when the problem gives none.
    pub failed: String,
    /// The folded line over finished thinking: `Thought for {count} {words}`.
    pub thought_for: String,
    /// The line while the model thinks, and alone before anything has come.
    pub thinking: String,
    pub markdown: MarkdownWords,
}

impl Default for AssistantReplyWords {
    fn default() -> Self {
        Self {
            name: "Assistant".into(),
            copy: "Copy".into(),
            copied: "Copied".into(),
            try_again: "Try again".into(),
            good: "Good answer".into(),
            bad: "Bad answer".into(),
            previous: "Previous version".into(),
            next: "Next version".into(),
            version: "{at} of {of}".into(),
            stopped: "Stopped".into(),
            failed: "The answer could not be finished".into(),
            thought_for: "Thought for {count} {words}".into(),
            thinking: "Thinking".into(),
            markdown: MarkdownWords::default(),
        }
    }
}

/// What a reply was asked this frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssistantReplyResponse {
    /// Copy was pressed; the answer is on the clipboard.
    pub copied: bool,
    /// Try again was pressed.
    pub retry: bool,
    /// Feedback was pressed: the feedback now, `None` once the given one is taken back.
    pub feedback: Option<Option<ReplyFeedback>>,
    /// Another version was asked for, counted from 1.
    pub version: Option<usize>,
}

/// The assistant's side of a turn, as sokkel-ui's `AssistantReply` in its `card` layout: one card
/// (`card` ground, a `border-strong` edge, `radius-surface`, 16 in and 16 between its parts) with
/// the speaker over it (its mark in a 32 square on `muted`, the name, the time muted), the model's
/// thinking folded behind a line counting its words, the answer in markdown, what the app adds
/// under it (`show`'s body: its usage line), a Stopped mark or what went wrong, and a foot on the
/// panel's ground with the versions, Copy, Try again and the feedback. While `streaming` the foot
/// waits; with nothing written yet the card says Thinking.
#[must_use = "show it with `.show(ui, |ui| ...)`"]
pub struct AssistantReply<'a> {
    id: Id,
    content: &'a str,
    name: Option<String>,
    code_name: bool,
    mark: Option<Icon>,
    time: Option<String>,
    time_title: Option<String>,
    reasoning: Option<&'a str>,
    state: AssistantState,
    problem: Option<String>,
    retry: bool,
    feedback: Option<ReplyFeedback>,
    feedback_offered: bool,
    versions: Option<(usize, usize)>,
    words: AssistantReplyWords,
}

impl<'a> AssistantReply<'a> {
    /// A reply whose answer is `content`, as markdown.
    pub fn new(id: impl egui::AsIdSalt, content: &'a str) -> Self {
        Self {
            id: Id::unique(id),
            content,
            name: None,
            code_name: false,
            mark: None,
            time: None,
            time_title: None,
            reasoning: None,
            state: AssistantState::Done,
            problem: None,
            retry: false,
            feedback: None,
            feedback_offered: false,
            versions: None,
            words: AssistantReplyWords::default(),
        }
    }

    /// Who speaks; `words.name` when none is given.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// The name in the code face, as a model's name is written.
    pub fn code_name(mut self, code: bool) -> Self {
        self.code_name = code;
        self
    }

    /// The assistant's mark, drawn in a square before the name.
    pub fn mark(mut self, icon: Icon) -> Self {
        self.mark = Some(icon);
        self
    }

    /// When the answer came, as the conversation shows it: `08:32`.
    pub fn time(mut self, time: impl Into<String>) -> Self {
        self.time = Some(time.into());
        self
    }

    /// The full date and time, on hover.
    pub fn time_title(mut self, title: impl Into<String>) -> Self {
        self.time_title = Some(title.into());
        self
    }

    /// The model's thinking, as markdown; none when empty.
    pub fn reasoning(mut self, text: &'a str) -> Self {
        self.reasoning = (!text.is_empty()).then_some(text);
        self
    }

    pub fn state(mut self, state: AssistantState) -> Self {
        self.state = state;
        self
    }

    /// What went wrong, when failed, in a sentence.
    pub fn problem(mut self, detail: impl Into<String>) -> Self {
        self.problem = Some(detail.into());
        self
    }

    /// Offers Try again, in the foot and on a failed answer.
    pub fn retry(mut self, on: bool) -> Self {
        self.retry = on;
        self
    }

    /// Offers Good answer and Bad answer, the given one drawn pressed.
    pub fn feedback(mut self, given: Option<ReplyFeedback>) -> Self {
        self.feedback = given;
        self.feedback_offered = true;
        self
    }

    /// Steps between the versions of an answer asked for again: the one shown from 1, and how many.
    pub fn versions(mut self, at: usize, of: usize) -> Self {
        self.versions = Some((at, of));
        self
    }

    pub fn words(mut self, words: AssistantReplyWords) -> Self {
        self.words = words;
        self
    }

    /// Draws the reply, `body` under the answer (its usage line), and says what was pressed.
    pub fn show(self, ui: &mut Ui, body: impl FnOnce(&mut Ui)) -> AssistantReplyResponse {
        let t = tokens(ui.ctx());
        let id = ui.make_persistent_id(self.id);
        let mut out = AssistantReplyResponse::default();
        let spacing = 16.0;
        let radius = t.radius("surface");
        let streaming = self.state == AssistantState::Streaming;
        let failed = self.state == AssistantState::Failed;
        let width = ui.available_width();
        Frame::new()
            .fill(t.color("card"))
            .stroke(Stroke::new(1.0, t.color("border-strong")))
            .corner_radius(radius)
            .show(ui, |ui| {
                ui.set_width(width - 2.0);
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                ui.vertical(|ui| {
                    ui.add_space(spacing);
                    Frame::new()
                        .inner_margin(Margin::symmetric(spacing as i8, 0))
                        .show(ui, |ui| {
                            ui.set_width(width - 2.0 - 2.0 * spacing);
                            ui.spacing_mut().item_spacing.y = spacing;
                            self.speaker(ui, &t);
                            self.answer(ui, &t, id, &mut out, body);
                        });
                    if streaming || failed {
                        ui.add_space(spacing);
                    } else {
                        ui.add_space(spacing);
                        self.foot(ui, &t, width - 2.0, radius, &mut out);
                    }
                });
            });
        out
    }

    fn speaker(&self, ui: &mut Ui, t: &Tokens) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if let Some(mark) = self.mark {
                let (rect, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                let radius = t.radius("control");
                ui.painter().rect(
                    rect,
                    radius,
                    t.color("muted"),
                    Stroke::new(1.0, t.color("border")),
                    StrokeKind::Inside,
                );
                icons::paint(ui.painter(), rect.center(), mark, 16.0, t.color("foreground"));
            }
            let name = self.name.clone().unwrap_or_else(|| self.words.name.clone());
            let body = TextRole::Body.font(t).size;
            let rich = if self.code_name {
                egui::RichText::new(name).font(mono_font(ui.ctx(), mono_size(t, body), t.weight(crate::Weight::Medium)))
            } else {
                TextRole::Body
                    .rich(t, name)
                    .variation("wght", t.weight(crate::Weight::Medium))
            };
            ui.add(egui::Label::new(rich.color(t.color("foreground"))).truncate());
            if let Some(time) = &self.time {
                let said = ui.label(TextRole::Help.rich(t, time).color(t.color("muted-foreground")));
                if let Some(title) = &self.time_title {
                    said.on_hover_text(title);
                }
            }
        });
    }

    fn answer(&self, ui: &mut Ui, t: &Tokens, id: Id, out: &mut AssistantReplyResponse, body: impl FnOnce(&mut Ui)) {
        let streaming = self.state == AssistantState::Streaming;
        let written = !self.content.is_empty();
        if streaming && !written && self.reasoning.is_none() {
            trace_line(ui, t, Icon::LoaderCircle, &self.words.thinking, false, false);
            ui.ctx().request_repaint_after_secs(0.5);
        }
        if let Some(text) = self.reasoning {
            reasoning(
                ui,
                id,
                text,
                streaming && !written,
                (&self.words.thinking, &self.words.thought_for),
                &self.words.markdown,
            );
        }
        if written {
            Markdown::new(self.content).words(self.words.markdown.clone()).show(ui);
        }
        body(ui);
        if self.state == AssistantState::Stopped {
            trace_line(ui, t, Icon::Square, &self.words.stopped, false, false);
        }
        if self.state == AssistantState::Failed {
            let mut problem = Problem::new(id.with("problem"), &self.words.failed);
            if let Some(detail) = &self.problem {
                problem = problem.detail(detail);
            }
            if self.retry {
                let mut act = |ui: &mut Ui| {
                    let again = Button::new(&self.words.try_again)
                        .icon(Icon::RotateCcw)
                        .size(ButtonSize::Sm)
                        .variant(ButtonVariant::Outline);
                    if ui.add(again).clicked() {
                        out.retry = true;
                    }
                };
                problem.show_with(ui, Some(&mut act));
            } else {
                problem.show(ui);
            }
        }
    }

    fn foot(&self, ui: &mut Ui, t: &Tokens, width: f32, radius: CornerRadius, out: &mut AssistantReplyResponse) {
        let spacing = 16.0;
        let height = t.density("control") + 2.0 * spacing;
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let bottom = CornerRadius { nw: 0, ne: 0, ..radius };
        ui.painter().rect_filled(rect, bottom, t.color("panel-ground"));
        ui.painter()
            .hline(rect.x_range(), rect.top() + 0.5, Stroke::new(1.0, t.color("border")));
        let inner = rect.shrink2(vec2(spacing, spacing));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(inner)
                .layout(Layout::left_to_right(Align::Center)),
        );
        let ui = &mut child;
        ui.spacing_mut().item_spacing.x = 8.0;
        let w = &self.words;
        if let Some((at, of)) = self.versions {
            ui.spacing_mut().item_spacing.x = 4.0;
            if ui
                .add(IconButton::new(Icon::ChevronLeft, &w.previous).md().enabled(at > 1))
                .clicked()
            {
                out.version = Some(at - 1);
            }
            let said = w
                .version
                .replace("{at}", &at.to_string())
                .replace("{of}", &of.to_string());
            ui.label(TextRole::Body.rich(t, said).color(t.color("muted-foreground")));
            if ui
                .add(IconButton::new(Icon::ChevronRight, &w.next).md().enabled(at < of))
                .clicked()
            {
                out.version = Some(at + 1);
            }
            ui.spacing_mut().item_spacing.x = 8.0;
        }
        let copied_id = self.id.with("copied");
        let now = ui.input(|i| i.time);
        let copied = ui
            .data(|d| d.get_temp::<f64>(copied_id))
            .is_some_and(|at| now - at < 1.5);
        let (icon, label) = if copied {
            (Icon::Check, &w.copied)
        } else {
            (Icon::Copy, &w.copy)
        };
        if ui.add(foot_button(label, icon)).clicked() {
            ui.ctx().copy_text(self.content.to_owned());
            ui.data_mut(|d| d.insert_temp(copied_id, now));
            out.copied = true;
        }
        if copied {
            ui.ctx().request_repaint_after_secs(0.25);
        }
        if self.retry && ui.add(foot_button(&w.try_again, Icon::RotateCcw)).clicked() {
            out.retry = true;
        }
        if self.feedback_offered {
            let good = self.feedback == Some(ReplyFeedback::Good);
            let bad = self.feedback == Some(ReplyFeedback::Bad);
            if ui
                .add(IconButton::new(Icon::ThumbsUp, &w.good).md().pressed(good))
                .clicked()
            {
                out.feedback = Some((!good).then_some(ReplyFeedback::Good));
            }
            if ui
                .add(IconButton::new(Icon::ThumbsDown, &w.bad).md().pressed(bad))
                .clicked()
            {
                out.feedback = Some((!bad).then_some(ReplyFeedback::Bad));
            }
        }
    }
}

/// A labeled ghost button of the reply's foot, its words and icon muted.
fn foot_button(label: &str, icon: Icon) -> Button {
    Button::new(label).icon(icon).variant(ButtonVariant::Ghost).muted(true)
}

// --- The person's message --------------------------------------------------------------------------

/// How a bubble is grounded, as sokkel-ui's `Bubble` `variant`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BubbleVariant {
    /// `primary` behind `primary-foreground`.
    #[default]
    Default,
    /// `secondary` behind `secondary-foreground`, as a person's question.
    Secondary,
    /// `muted` behind the foreground.
    Muted,
}

/// A message's words in a bubble, as sokkel-ui's `Bubble`: at most 80 percent of the row, 12 by 8
/// in, `radius-xl`, the body size at the relaxed leading, at the row's end with `end`.
#[must_use = "add it with `ui.add(...)`"]
pub struct Bubble {
    text: String,
    variant: BubbleVariant,
    end: bool,
}

impl Bubble {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            variant: BubbleVariant::Default,
            end: false,
        }
    }

    pub fn variant(mut self, variant: BubbleVariant) -> Self {
        self.variant = variant;
        self
    }

    /// At the row's end, as the person's own messages stand.
    pub fn end(mut self, end: bool) -> Self {
        self.end = end;
        self
    }
}

impl egui::Widget for Bubble {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let (ground, ink) = match self.variant {
            BubbleVariant::Default => (t.color("primary"), t.color("primary-foreground")),
            BubbleVariant::Secondary => (t.color("secondary"), t.color("secondary-foreground")),
            BubbleVariant::Muted => (t.color("muted"), t.color("foreground")),
        };
        let row = ui.available_width();
        let room = (row * 0.8 - 24.0).max(1.0);
        let body = t.text("body");
        let rich = TextRole::Body
            .rich(&t, &self.text)
            .line_height(Some((body.size * 1.625).round()));
        let galley = WidgetText::from(rich).into_galley(ui, Some(TextWrapMode::Wrap), room, TextRole::Body.font(&t));
        let size = galley.size() + vec2(24.0, 16.0) + vec2(2.0, 2.0);
        let (row_rect, response) = ui.allocate_exact_size(vec2(row, size.y), Sense::hover());
        let left = if self.end {
            row_rect.right() - size.x
        } else {
            row_rect.left()
        };
        let rect = Rect::from_min_size(pos2(left, row_rect.top()), size);
        ui.painter().rect_filled(rect, t.radius("xl"), ground);
        ui.painter().galley(rect.min + vec2(13.0, 9.0), galley, ink);
        response
    }
}

/// The words reaction chips say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReactionChipsWords {
    /// The button that adds one.
    pub react: String,
}

impl Default for ReactionChipsWords {
    fn default() -> Self {
        Self {
            react: "Add a reaction".into(),
        }
    }
}

/// The reactions under a message, as sokkel-ui's `ReactionChips`: each emoji with its count, 24
/// tall, 6 across, `radius-control`, on `muted` in `muted-foreground`, the viewer's own on
/// `primary/12` edged `primary/60` in `primary-ink`; then a chip-shaped button holding a 14
/// smile-plus that offers the quick reactions. 4 apart, wrapping, at the row's end with `end`.
#[must_use = "show it with `.show(ui)`"]
pub struct ReactionChips<'a> {
    id: Id,
    reactions: &'a [Reaction],
    add: bool,
    end: bool,
    words: ReactionChipsWords,
}

impl<'a> ReactionChips<'a> {
    pub fn new(id: impl egui::AsIdSalt, reactions: &'a [Reaction]) -> Self {
        Self {
            id: Id::unique(id),
            reactions,
            add: true,
            end: false,
            words: ReactionChipsWords::default(),
        }
    }

    /// Offers the add button; on by default.
    pub fn addable(mut self, add: bool) -> Self {
        self.add = add;
        self
    }

    pub fn end(mut self, end: bool) -> Self {
        self.end = end;
        self
    }

    pub fn words(mut self, words: ReactionChipsWords) -> Self {
        self.words = words;
        self
    }

    /// Draws the chips; a reaction pressed or picked: its emoji and its name.
    pub fn show(self, ui: &mut Ui) -> Option<(String, String)> {
        let t = tokens(ui.ctx());
        let layout = if self.end {
            Layout::right_to_left(Align::Center)
        } else {
            Layout::left_to_right(Align::Center)
        };
        ui.allocate_ui_with_layout(vec2(ui.available_width(), 24.0), layout, |ui| {
            let mut out = None;
            ui.spacing_mut().item_spacing.x = 4.0;
            let mut chips: Vec<&Reaction> = self.reactions.iter().collect();
            if self.end {
                chips.reverse();
                if self.add {
                    out = out.or(self.add_button(ui, &t));
                }
            }
            for reaction in chips {
                if reaction_chip(ui, &t, reaction).clicked() {
                    out = Some((reaction.id.clone(), reaction.label.clone()));
                }
            }
            if !self.end && self.add {
                out = out.or(self.add_button(ui, &t));
            }
            out
        })
        .inner
    }

    fn add_button(&self, ui: &mut Ui, t: &Tokens) -> Option<(String, String)> {
        let (rect, _) = ui.allocate_exact_size(vec2(26.0, 24.0), Sense::hover());
        let id = ui.make_persistent_id(self.id).with("add");
        let response = ui.interact(rect, id, Sense::click()).on_hover_text(&self.words.react);
        let open = egui::Popup::is_id_open(ui.ctx(), response.id.with("menu"));
        let hot = response.hovered() || open;
        ui.painter().rect(
            rect,
            t.radius("control"),
            t.color("muted"),
            Stroke::new(
                1.0,
                if hot {
                    t.color("border-strong")
                } else {
                    Color32::TRANSPARENT
                },
            ),
            StrokeKind::Inside,
        );
        icons::paint(
            ui.painter(),
            rect.center(),
            Icon::SmilePlus,
            14.0,
            if hot {
                t.color("foreground")
            } else {
                t.color("muted-foreground")
            },
        );
        let quick: Vec<MenuItem> = QUICK_REACTIONS
            .iter()
            .map(|(emoji, name)| MenuItem::action(format!("{emoji}  {name}")))
            .collect();
        DropdownMenu::new(&quick).width(200.0).show(&response).map(|index| {
            let (emoji, name) = QUICK_REACTIONS[index];
            (emoji.to_owned(), name.to_owned())
        })
    }
}

fn reaction_chip(ui: &mut Ui, t: &Tokens, reaction: &Reaction) -> Response {
    let emoji = WidgetText::from(TextRole::Body.rich(t, &reaction.id)).into_galley(
        ui,
        Some(TextWrapMode::Extend),
        f32::INFINITY,
        TextRole::Body.font(t),
    );
    let count = WidgetText::from(TextRole::Badge.rich(t, usage_count(reaction.count as f64))).into_galley(
        ui,
        Some(TextWrapMode::Extend),
        f32::INFINITY,
        TextRole::Badge.font(t),
    );
    let size = vec2(6.0 + emoji.size().x + 4.0 + count.size().x + 6.0, 24.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let response = response.on_hover_text(&reaction.label);
    let (ground, edge, ink) = if reaction.mine {
        (
            t.color_at("primary", 0.12),
            t.color_at("primary", 0.6),
            t.color("primary-ink"),
        )
    } else {
        (
            t.color("muted"),
            if response.hovered() {
                t.color("border-strong")
            } else {
                Color32::TRANSPARENT
            },
            t.color("muted-foreground"),
        )
    };
    ui.painter().rect(
        rect,
        t.radius("control"),
        ground,
        Stroke::new(1.0, edge),
        StrokeKind::Inside,
    );
    let emoji_width = emoji.size().x;
    ui.painter().galley(
        pos2(rect.left() + 6.0, rect.center().y - emoji.size().y / 2.0),
        emoji,
        ink,
    );
    ui.painter().galley(
        pos2(rect.left() + 10.0 + emoji_width, rect.center().y - count.size().y / 2.0),
        count,
        ink,
    );
    response
}

// --- Answer column ---------------------------------------------------------------------------------

/// Where one model's answer is: asked and not yet begun, being written, written, stopped part way,
/// or failed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnswerState {
    Waiting,
    Streaming,
    #[default]
    Done,
    Stopped,
    Failed,
}

/// The words an answer column says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnswerColumnWords {
    pub waiting: String,
    pub streaming: String,
    pub done: String,
    pub stopped: String,
    pub failed: String,
    /// Said in the body before the first token.
    pub first: String,
    pub copy: String,
    pub copied: String,
    /// The button that asks the model again.
    pub again: String,
    pub stop: String,
    pub remove: String,
    /// The title of a failed answer.
    pub failed_title: String,
    pub thinking: String,
    /// `Thought for {count} {words}`.
    pub thought_for: String,
    pub usage: UsageLineWords,
    pub markdown: MarkdownWords,
}

impl Default for AnswerColumnWords {
    fn default() -> Self {
        Self {
            waiting: "waiting".into(),
            streaming: "streaming".into(),
            done: "done".into(),
            stopped: "stopped".into(),
            failed: "failed".into(),
            first: "Waiting for the first token".into(),
            copy: "Copy the answer".into(),
            copied: "Copied".into(),
            again: "Run again".into(),
            stop: "Stop".into(),
            remove: "Remove".into(),
            failed_title: "The model could not answer".into(),
            thinking: "Thinking".into(),
            thought_for: "Thought for {count} {words}".into(),
            usage: UsageLineWords::default(),
            markdown: MarkdownWords::default(),
        }
    }
}

/// What an answer column was asked this frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AnswerColumnResponse {
    pub copied: bool,
    pub stop: bool,
    pub retry: bool,
    pub remove: bool,
}

/// The figures of an answer's usage line, for a part that draws one of its own.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UsageFigures {
    pub input: Option<f64>,
    pub output: f64,
    pub first_token: Option<f64>,
    pub rate: Option<f64>,
    pub latency: Option<f64>,
    pub note: Option<String>,
}

impl UsageFigures {
    /// The usage line of these figures.
    pub fn line(&self) -> UsageLine {
        UsageLine {
            input: self.input,
            output: self.output,
            first_token: self.first_token,
            rate: self.rate,
            latency: self.latency,
            note: self.note.clone(),
            live: false,
            words: UsageLineWords::default(),
        }
    }
}

/// One model's answer to a prompt sent to several, as sokkel-ui's `AnswerColumn`: a card edged in
/// `border-strong` with the model in the code face over a short fact on the section heads' ground,
/// the state as a chip and Copy, Stop, Run again and Remove at its end; then the thinking, the
/// answer in markdown, a failure, and the usage line under a rule at its foot. At least
/// `min_height` tall (320 by default, as the web's `min-h-80`).
#[must_use = "show it with `.show(ui)`"]
pub struct AnswerColumn<'a> {
    id: Id,
    model: String,
    detail: Option<String>,
    state: AnswerState,
    content: &'a str,
    reasoning: &'a str,
    usage: Option<UsageFigures>,
    problem: Option<String>,
    stop: bool,
    retry: bool,
    remove: bool,
    min_height: f32,
    words: AnswerColumnWords,
}

impl<'a> AnswerColumn<'a> {
    pub fn new(id: impl egui::AsIdSalt, model: impl Into<String>, state: AnswerState, content: &'a str) -> Self {
        Self {
            id: Id::unique(id),
            model: model.into(),
            detail: None,
            state,
            content,
            reasoning: "",
            usage: None,
            problem: None,
            stop: false,
            retry: false,
            remove: false,
            min_height: 320.0,
            words: AnswerColumnWords::default(),
        }
    }

    /// A short fact under the model: its rounding and size.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// The model's thinking, as markdown.
    pub fn reasoning(mut self, text: &'a str) -> Self {
        self.reasoning = text;
        self
    }

    pub fn usage(mut self, usage: UsageFigures) -> Self {
        self.usage = Some(usage);
        self
    }

    /// What went wrong, when failed.
    pub fn problem(mut self, detail: impl Into<String>) -> Self {
        self.problem = Some(detail.into());
        self
    }

    /// Offers Stop while the answer is waited for or written.
    pub fn stop(mut self, on: bool) -> Self {
        self.stop = on;
        self
    }

    /// Offers Run again once the answer has ended.
    pub fn retry(mut self, on: bool) -> Self {
        self.retry = on;
        self
    }

    /// Offers Remove.
    pub fn remove(mut self, on: bool) -> Self {
        self.remove = on;
        self
    }

    pub fn min_height(mut self, height: f32) -> Self {
        self.min_height = height;
        self
    }

    pub fn words(mut self, words: AnswerColumnWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> AnswerColumnResponse {
        let t = tokens(ui.ctx());
        let id = ui.make_persistent_id(self.id);
        let mut out = AnswerColumnResponse::default();
        let width = ui.available_width();
        let pad = t.density("panel-pad");
        let going = matches!(self.state, AnswerState::Waiting | AnswerState::Streaming);
        let w = &self.words;
        let radius = t.radius("surface");
        let shown = Frame::new().fill(t.color("card")).corner_radius(radius).show(ui, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ui.vertical(|ui| {
                // the head: the model over its fact, the chip and the buttons at its end
                let under = ui.painter().add(egui::Shape::Noop);
                let head = Frame::new()
                    .inner_margin(Margin::symmetric(pad as i8, 8))
                    .show(ui, |ui| {
                        ui.set_width(width - 2.0 * pad);
                        let lines = TextRole::Head.line_height(&t) + TextRole::Help.line_height(&t);
                        let row = vec2(width - 2.0 * pad, lines);
                        ui.allocate_ui_with_layout(row, Layout::right_to_left(Align::Center), |ui| {
                            ui.set_min_size(row);
                            ui.spacing_mut().item_spacing.x = 4.0;
                            if self.remove && ui.add(IconButton::new(Icon::X, &w.remove)).clicked() {
                                out.remove = true;
                            }
                            if !going && self.retry && ui.add(IconButton::new(Icon::RotateCcw, &w.again)).clicked() {
                                out.retry = true;
                            }
                            if going && self.stop && ui.add(IconButton::new(Icon::Square, &w.stop)).clicked() {
                                out.stop = true;
                            }
                            if !self.content.is_empty() {
                                let copied_id = id.with("copied");
                                let now = ui.input(|i| i.time);
                                let copied = ui
                                    .data(|d| d.get_temp::<f64>(copied_id))
                                    .is_some_and(|at| now - at < 1.5);
                                let (icon, label) = if copied {
                                    (Icon::Check, &w.copied)
                                } else {
                                    (Icon::Copy, &w.copy)
                                };
                                if ui.add(IconButton::new(icon, label)).clicked() {
                                    ui.ctx().copy_text(self.content.to_owned());
                                    ui.data_mut(|d| d.insert_temp(copied_id, now));
                                    out.copied = true;
                                }
                            }
                            let (label, status) = match self.state {
                                AnswerState::Waiting => (&w.waiting, "queued"),
                                AnswerState::Streaming => (&w.streaming, "running"),
                                AnswerState::Done => (&w.done, "succeeded"),
                                AnswerState::Stopped => (&w.stopped, "cancelled"),
                                AnswerState::Failed => (&w.failed, "failed"),
                            };
                            ui.add(Chip::new(label).tone(ChipTone::Status(status)));
                            ui.add_space(8.0);
                            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                let size = TextRole::Head.font(&t).size;
                                let name = egui::RichText::new(&self.model)
                                    .font(mono_font(
                                        ui.ctx(),
                                        mono_size(&t, size),
                                        t.weight(crate::Weight::Semibold),
                                    ))
                                    .color(t.color_or("section-head-ink", t.color("foreground")));
                                ui.add(egui::Label::new(name).truncate());
                                if let Some(detail) = &self.detail {
                                    ui.add(
                                        egui::Label::new(
                                            TextRole::Help.rich(&t, detail).color(t.color("muted-foreground")),
                                        )
                                        .truncate(),
                                    );
                                }
                            });
                        });
                    });
                let head_rect = head.response.rect;
                let top_radius = CornerRadius { sw: 0, se: 0, ..radius };
                ui.painter().set(
                    under,
                    egui::Shape::rect_filled(
                        head_rect,
                        top_radius,
                        t.color_or("section-head-ground", t.color("muted")),
                    ),
                );
                ui.painter().hline(
                    head_rect.x_range(),
                    head_rect.bottom() - 0.5,
                    Stroke::new(1.0, t.color_or("section-head-line", t.color("border"))),
                );
                // the body: the thinking, the answer and what went wrong
                let body_min =
                    (self.min_height - head_rect.height() - if self.usage.is_some() { 41.0 } else { 0.0 }).max(0.0);
                Frame::new().inner_margin(Margin::same(pad as i8)).show(ui, |ui| {
                    ui.set_width(width - 2.0 * pad);
                    ui.set_min_height((body_min - 2.0 * pad).max(0.0));
                    ui.spacing_mut().item_spacing.y = 12.0;
                    if !self.reasoning.is_empty() {
                        reasoning(
                            ui,
                            id,
                            self.reasoning,
                            going && self.content.is_empty(),
                            (&w.thinking, &w.thought_for),
                            &w.markdown,
                        );
                    }
                    if self.state == AnswerState::Waiting && self.content.is_empty() && self.reasoning.is_empty() {
                        ui.label(TextRole::Body.rich(&t, &w.first).color(t.color("muted-foreground")));
                    }
                    if !self.content.is_empty() {
                        Markdown::new(self.content).words(w.markdown.clone()).show(ui);
                    }
                    if self.state == AnswerState::Failed {
                        let mut problem = Problem::new(id.with("problem"), &w.failed_title);
                        if let Some(detail) = &self.problem {
                            problem = problem.detail(detail);
                        }
                        problem.show(ui);
                    }
                });
                if let Some(usage) = &self.usage {
                    let rule = ui.cursor().top();
                    ui.painter()
                        .hline(ui.max_rect().x_range(), rule + 0.5, Stroke::new(1.0, t.color("border")));
                    Frame::new()
                        .inner_margin(Margin::symmetric(pad as i8, 10))
                        .show(ui, |ui| {
                            ui.set_width(width - 2.0 * pad);
                            ui.add(UsageLine {
                                live: going,
                                words: w.usage.clone(),
                                ..usage.line()
                            });
                        });
                }
            });
        });
        let rect = shown.response.rect;
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, t.color("border-strong")),
            StrokeKind::Inside,
        );
        out
    }
}

// --- Conversation ----------------------------------------------------------------------------------

/// A conversation's column, as sokkel-ui's `Conversation` lays out an assistant's chat: the header
/// on top, the messages scrolling under it from their end (it opens at the latest and follows what
/// is added while the reader is at the end), centered at most `max_width` wide (768, the web's
/// `max-w-3xl`) with 16 at their sides and 24 over and under, and the composer at the foot, as
/// wide, 16 over the column's foot.
#[must_use = "show it with `.show(ui, ...)`"]
pub struct Conversation {
    id: Id,
    max_width: f32,
}

impl Conversation {
    pub fn new(id: impl egui::AsIdSalt) -> Self {
        Self {
            id: Id::unique(id),
            max_width: 768.0,
        }
    }

    /// The widest the messages and the composer stand.
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = width;
        self
    }

    /// Draws the column in the room given: `header` across its top, `messages` in the scroll, and
    /// `composer` at its foot.
    pub fn show(
        self,
        ui: &mut Ui,
        header: impl FnOnce(&mut Ui),
        messages: impl FnOnce(&mut Ui),
        composer: impl FnOnce(&mut Ui),
    ) {
        let full = ui.available_rect_before_wrap();
        let width = full.width();
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        ui.vertical(|ui| {
            ui.set_width(width);
            header(ui);
            let inner = (width - 32.0).min(self.max_width - 32.0).max(1.0);
            let side = ((width - inner) / 2.0).max(0.0);
            // the composer is measured on the frame before, so the scroll ends over it
            let foot_id = ui.make_persistent_id(self.id).with("foot");
            let foot: f32 = ui.data(|d| d.get_temp(foot_id)).unwrap_or(64.0);
            let scroll_height = (full.bottom() - ui.cursor().top() - foot - 8.0).max(40.0);
            egui::ScrollArea::vertical()
                .id_salt(self.id.with("messages"))
                .max_height(scroll_height)
                .min_scrolled_height(scroll_height)
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(width);
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.add_space(side);
                        ui.vertical(|ui| {
                            ui.set_width(inner);
                            ui.add_space(24.0);
                            messages(ui);
                            ui.add_space(24.0 + 12.0);
                        });
                    });
                });
            ui.add_space(8.0);
            let side = ((width - (self.max_width.min(width - 32.0))) / 2.0).max(16.0);
            let top = ui.cursor().top();
            ui.horizontal(|ui| {
                ui.add_space(side);
                ui.vertical(|ui| {
                    ui.set_width(width - 2.0 * side);
                    composer(ui);
                });
            });
            ui.add_space(16.0);
            let used = ui.cursor().top() - top;
            ui.data_mut(|d| d.insert_temp(foot_id, used));
        });
    }
}

// --- Composer --------------------------------------------------------------------------------------

/// How a composer stands: the text over a row of controls, or one line that opens into a box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ComposerLayout {
    #[default]
    Stacked,
    Inline,
}

/// What a file waiting to go is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AttachmentKind {
    Image,
    #[default]
    File,
}

/// A file waiting to go with the next message.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComposerAttachment {
    pub id: String,
    pub name: String,
    pub kind: AttachmentKind,
    /// A short fact under the name: `CSV, 2.1 MB`.
    pub detail: Option<String>,
    /// Still on its way up: a spinner, and Send waits.
    pub uploading: bool,
}

/// The words a composer says, as sokkel-design's `composer` spec has them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposerWords {
    pub placeholder: String,
    pub send: String,
    pub stop: String,
    pub attach: String,
    pub remove: String,
    pub uploading: String,
    pub waiting: String,
}

impl Default for ComposerWords {
    fn default() -> Self {
        Self {
            placeholder: "Ask anything".into(),
            send: "Send".into(),
            stop: "Stop".into(),
            attach: "Attach files".into(),
            remove: "Remove".into(),
            uploading: "Uploading".into(),
            waiting: "Waiting for files to upload".into(),
        }
    }
}

/// What a composer was asked this frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComposerResponse {
    /// Enter or Send while ready: the app sends the text and empties it.
    pub send: bool,
    /// Stop while busy.
    pub stop: bool,
    /// The attach button: the app opens its file picker.
    pub attach: bool,
    /// A file's remove button, by the file's id.
    pub removed: Option<String>,
    /// The text changed.
    pub changed: bool,
}

/// Where a message to an assistant is written, after sokkel-design's `composer`: a box on the
/// field's ground edged in `border-strong` with `elevation-surface` under it, rounded by
/// `radius-composer`, the edge and a 1 wide ring in `ring` while its text has focus. Stacked
/// (the default) the text is on top, at least 48 tall, and the controls in a row under it: attach,
/// the app's tools, and a round Send, Stop in its place while `busy`. Inline it is one line, a pill
/// whose corners are half its height plus 12, with a round secondary plus, the text, the tools and
/// Send; it opens into the box once the text (or the placeholder) is wider than the room the
/// controls leave or breaks a line, and closes once it fits again with 8 to spare. Enter sends,
/// Shift+Enter breaks the line. The files waiting to go stand over the text as chips.
#[must_use = "show it with `.show(ui, |ui| ...)`"]
pub struct Composer<'a> {
    id: Id,
    text: &'a mut String,
    layout: ComposerLayout,
    busy: bool,
    stop: bool,
    disabled: bool,
    attach: bool,
    attachments: &'a [ComposerAttachment],
    words: ComposerWords,
}

impl<'a> Composer<'a> {
    pub fn new(id: impl egui::AsIdSalt, text: &'a mut String) -> Self {
        Self {
            id: Id::unique(id),
            text,
            layout: ComposerLayout::Stacked,
            busy: false,
            stop: true,
            disabled: false,
            attach: false,
            attachments: &[],
            words: ComposerWords::default(),
        }
    }

    pub fn layout(mut self, layout: ComposerLayout) -> Self {
        self.layout = layout;
        self
    }

    /// An answer is streaming: Send becomes Stop.
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    /// Offers Stop while busy; on by default.
    pub fn stoppable(mut self, on: bool) -> Self {
        self.stop = on;
        self
    }

    /// Shuts the text and every control; the box at 60 percent.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Offers the attach button, as the web's `onAttach`.
    pub fn attach(mut self, on: bool) -> Self {
        self.attach = on;
        self
    }

    pub fn attachments(mut self, files: &'a [ComposerAttachment]) -> Self {
        self.attachments = files;
        self
    }

    pub fn words(mut self, words: ComposerWords) -> Self {
        self.words = words;
        self
    }

    /// Draws the composer, `tools` (the app's controls, a model's select) after attach.
    pub fn show(mut self, ui: &mut Ui, tools: impl FnOnce(&mut Ui)) -> ComposerResponse {
        let t = tokens(ui.ctx());
        let id = ui.make_persistent_id(self.id);
        let field_id = id.with("text");
        let mut out = ComposerResponse::default();
        let uploading = self.attachments.iter().any(|file| file.uploading);
        let has_text = !self.text.trim().is_empty();
        let ready = !self.disabled && !self.busy && !uploading && (has_text || !self.attachments.is_empty());
        // Enter sends before the text takes it as a line break; Shift+Enter breaks the line
        let focused = ui.memory(|m| m.has_focus(field_id));
        if focused && !self.disabled && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) && ready {
            out.send = true;
        }
        let inline = self.layout == ComposerLayout::Inline;
        let opened_id = id.with("opened");
        let was_open: bool = ui.data(|d| d.get_temp(opened_id)).unwrap_or(false);
        let control = if inline {
            t.density("control")
        } else {
            t.density("control-sm")
        };
        let width = ui.available_width();
        let body = TextRole::Body.font(&t);
        // inline, the room the text has beside the controls on one line, the tools measured last frame
        let tools_id = id.with("tools-width");
        let tools_width: f32 = ui.data(|d| d.get_temp(tools_id)).unwrap_or(0.0);
        let pad = 6.0;
        let gap = 6.0;
        let line_room = width - 2.0 * pad - 2.0 * control - tools_width - 3.0 * gap - 12.0;
        let measured = {
            let said = if self.text.is_empty() {
                &self.words.placeholder
            } else {
                &*self.text
            };
            ui.fonts_mut(|f| f.layout_no_wrap(said.to_owned(), body.clone(), Color32::WHITE).size().x)
        };
        let open = inline
            && (self.text.contains('\n')
                || !self.attachments.is_empty()
                || measured > line_room
                || (was_open && measured > line_room - 8.0));
        if inline {
            ui.data_mut(|d| d.insert_temp(opened_id, open));
        }
        let pill = inline && !open;
        let radius = if pill {
            let height = control + 2.0 * pad;
            CornerRadius::same((height / 2.0 + 12.0).min(255.0) as u8)
        } else {
            t.radius("composer")
        };
        let edge = if focused {
            t.color("ring")
        } else {
            t.color("border-strong")
        };
        let opacity = if self.disabled { 0.6 } else { 1.0 };
        let frame = Frame::new()
            .fill(t.color("field").gamma_multiply(opacity))
            .stroke(Stroke::new(1.0, edge.gamma_multiply(opacity)))
            .shadow(t.shadow("elevation-surface"))
            .corner_radius(radius)
            .inner_margin(if inline { Margin::same(pad as i8) } else { Margin::ZERO });
        let shown = frame.show(ui, |ui| {
            let inner = if inline { width - 2.0 * pad - 2.0 } else { width - 2.0 };
            ui.set_width(inner);
            ui.spacing_mut().item_spacing = vec2(gap, 0.0);
            if !self.attachments.is_empty() {
                let inset = if inline { 0.0 } else { 12.0 };
                ui.add_space(if inline { 0.0 } else { 12.0 });
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(inset);
                    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                    for file in self.attachments {
                        if file_chip(ui, &t, file, &self.words) {
                            out.removed = Some(file.id.clone());
                        }
                    }
                });
                if inline {
                    ui.add_space(6.0);
                }
            }
            if pill {
                ui.horizontal(|ui| {
                    ui.set_height(control);
                    if self.attach && self.round_attach(ui, &t, control, true) {
                        out.attach = true;
                    }
                    let room = (ui.available_width() - control - tools_width - 2.0 * gap).max(40.0);
                    self.text_field(ui, &t, field_id, room, true, &mut out);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        self.send_or_stop(ui, &t, control, ready, uploading, true, &mut out);
                        let before = ui.cursor().right();
                        ui.add_enabled_ui(!self.disabled, tools);
                        let used = (before - ui.cursor().right() - gap).max(0.0);
                        ui.data_mut(|d| d.insert_temp(tools_id, used));
                    });
                });
            } else {
                // the text on a line of its own, the controls under it
                if inline {
                    self.text_field(ui, &t, field_id, ui.available_width(), true, &mut out);
                    ui.add_space(4.0);
                } else {
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        let room = ui.available_width() - 12.0 - ui.spacing().item_spacing.x;
                        self.text_field(ui, &t, field_id, room, false, &mut out);
                    });
                    ui.add_space(4.0);
                }
                ui.horizontal(|ui| {
                    ui.set_height(control);
                    if !inline {
                        ui.add_space(8.0);
                        ui.spacing_mut().item_spacing.x = 4.0;
                    }
                    if self.attach && self.round_attach(ui, &t, control, inline) {
                        out.attach = true;
                    }
                    let before = ui.cursor().left();
                    ui.add_enabled_ui(!self.disabled, |ui| {
                        ui.horizontal(|ui| tools(ui));
                    });
                    let used = (ui.cursor().left() - before - gap).max(0.0);
                    ui.data_mut(|d| d.insert_temp(tools_id, used));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if !inline {
                            ui.add_space(8.0);
                        }
                        self.send_or_stop(ui, &t, control, ready, uploading, inline, &mut out);
                    });
                });
                if !inline {
                    ui.add_space(8.0);
                }
            }
        });
        let rect = shown.response.rect;
        if focused {
            ui.painter().rect_stroke(
                rect.expand(1.0),
                radius,
                Stroke::new(1.0, t.color("ring")),
                StrokeKind::Inside,
            );
        }
        // a press on the box's ground puts the caret in the text
        let ground = ui.interact(rect, id.with("ground"), Sense::click());
        if ground.clicked() && !self.disabled {
            ui.memory_mut(|m| m.request_focus(field_id));
        }
        if out.send {
            ui.memory_mut(|m| m.request_focus(field_id));
        }
        out
    }

    fn text_field(
        &mut self,
        ui: &mut Ui,
        t: &Tokens,
        field_id: Id,
        room: f32,
        inline: bool,
        out: &mut ComposerResponse,
    ) {
        let body = TextRole::Body.font(t);
        let hint = TextRole::Body
            .rich(t, &self.words.placeholder)
            .color(t.color("muted-foreground"));
        ui.allocate_ui_with_layout(vec2(room, 240.0), Layout::top_down(Align::Min), |ui| {
            ui.set_width(room);
            egui::ScrollArea::vertical()
                .id_salt(field_id.with("scroll"))
                .max_height(240.0)
                .min_scrolled_height(0.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    let room = ui.available_width();
                    let edit = TextEdit::multiline(self.text)
                        .id(field_id)
                        .hint_text(hint)
                        .font(body)
                        .text_color(t.color("foreground"))
                        .desired_rows(1)
                        .desired_width(if inline { room - 12.0 } else { room })
                        .frame(Frame::NONE.inner_margin(if inline { Margin::symmetric(6, 4) } else { Margin::ZERO }))
                        .interactive(!self.disabled);
                    let edited = ui.add(edit);
                    if !inline {
                        let rest = (48.0 - 16.0 - edited.rect.height()).max(0.0);
                        ui.add_space(rest);
                    }
                    if edited.changed() {
                        out.changed = true;
                    }
                });
        });
    }

    fn round_attach(&self, ui: &mut Ui, t: &Tokens, side: f32, inline: bool) -> bool {
        let sense = if self.disabled { Sense::hover() } else { Sense::click() };
        let (rect, response) = ui.allocate_exact_size(vec2(side, side), sense);
        let response = response.on_hover_text(&self.words.attach);
        let hot = response.hovered() && !self.disabled;
        let ground = if inline {
            if hot { t.color("hover") } else { t.color("secondary") }
        } else if hot {
            t.color("hover")
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().circle_filled(rect.center(), side / 2.0, ground);
        let ink = if inline || hot {
            t.color("foreground")
        } else {
            t.color("muted-foreground")
        };
        let (icon, size) = if inline {
            (Icon::Plus, 16.0)
        } else {
            (Icon::Paperclip, 16.0)
        };
        icons::paint(ui.painter(), rect.center(), icon, size, dim(ink, !self.disabled));
        response.clicked()
    }

    #[allow(clippy::too_many_arguments)]
    fn send_or_stop(
        &self,
        ui: &mut Ui,
        t: &Tokens,
        side: f32,
        ready: bool,
        uploading: bool,
        inline: bool,
        out: &mut ComposerResponse,
    ) {
        if self.busy && self.stop {
            let (rect, response) = ui.allocate_exact_size(vec2(side, side), Sense::click());
            let response = response.on_hover_text(&self.words.stop);
            let (ground, ink) = if inline {
                (t.color("primary"), t.color("primary-foreground"))
            } else {
                (t.color("secondary"), t.color("secondary-foreground"))
            };
            let ground = if response.hovered() {
                ground.gamma_multiply(0.8)
            } else {
                ground
            };
            ui.painter().circle_filled(rect.center(), side / 2.0, ground);
            ui.painter()
                .rect_filled(Rect::from_center_size(rect.center(), vec2(10.0, 10.0)), 2.0, ink);
            if response.clicked() {
                out.stop = true;
            }
            return;
        }
        let sense = if ready { Sense::click() } else { Sense::hover() };
        let (rect, response) = ui.allocate_exact_size(vec2(side, side), sense);
        let label = if uploading {
            &self.words.waiting
        } else {
            &self.words.send
        };
        let response = response.on_hover_text(label);
        let ground = if response.hovered() && ready {
            t.color_at("primary", 0.8)
        } else {
            t.color("primary")
        };
        let alpha = if ready { 1.0 } else { DISABLED };
        ui.painter()
            .circle_filled(rect.center(), side / 2.0, ground.gamma_multiply(alpha));
        icons::paint(
            ui.painter(),
            rect.center(),
            Icon::ArrowUp,
            16.0,
            t.color("primary-foreground").gamma_multiply(alpha),
        );
        if response.clicked() && ready {
            out.send = true;
        }
    }
}

/// A file waiting to go: 56 tall, at most 224 wide, on the card edged in `border`, `radius-md`, its
/// icon (or the upload's spinner), its name in the badge role and its detail muted, a 20 round
/// remove at its top end corner under the pointer; whether remove was pressed.
fn file_chip(ui: &mut Ui, t: &Tokens, file: &ComposerAttachment, words: &ComposerWords) -> bool {
    let name = WidgetText::from(TextRole::Badge.rich(t, &file.name)).into_galley(
        ui,
        Some(TextWrapMode::Truncate),
        224.0 - 20.0 - 16.0 - 8.0,
        TextRole::Badge.font(t),
    );
    let detail_said = if file.uploading {
        words.uploading.clone()
    } else {
        file.detail.clone().unwrap_or_default()
    };
    let detail = WidgetText::from(TextRole::Help.rich(t, detail_said)).into_galley(
        ui,
        Some(TextWrapMode::Truncate),
        224.0 - 20.0 - 16.0 - 8.0,
        TextRole::Help.font(t),
    );
    let width = (10.0 + 16.0 + 8.0 + name.size().x.max(detail.size().x) + 10.0).min(224.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 56.0), Sense::hover());
    ui.painter().rect(
        rect,
        t.radius("md"),
        t.color("card"),
        Stroke::new(1.0, t.color("border")),
        StrokeKind::Inside,
    );
    let icon_at = pos2(rect.left() + 18.0, rect.center().y);
    if file.uploading {
        spinner(ui, ui.painter(), icon_at, 7.0, t.color("muted-foreground"));
    } else {
        let icon = match file.kind {
            AttachmentKind::Image => Icon::Image,
            AttachmentKind::File => Icon::FileText,
        };
        icons::paint(ui.painter(), icon_at, icon, 16.0, t.color("muted-foreground"));
    }
    let x = rect.left() + 34.0;
    let lines = name.size().y + detail.size().y;
    let top = rect.center().y - lines / 2.0;
    let name_height = name.size().y;
    ui.painter().galley(pos2(x, top), name, t.color("foreground"));
    ui.painter()
        .galley(pos2(x, top + name_height), detail, t.color("muted-foreground"));
    let near = ui.rect_contains_pointer(rect.expand(8.0));
    let mut removed = false;
    if near || response.hovered() {
        let spot = Rect::from_center_size(pos2(rect.right() - 2.0, rect.top() + 2.0), vec2(20.0, 20.0));
        let hit = ui
            .interact(spot, response.id.with(("remove", &file.id)), Sense::click())
            .on_hover_text(format!("{} {}", words.remove, file.name));
        ui.painter().circle(
            spot.center(),
            10.0,
            t.color("popover"),
            Stroke::new(1.0, t.color("border-strong")),
        );
        let ink = if hit.hovered() {
            t.color("foreground")
        } else {
            t.color("muted-foreground")
        };
        icons::paint(ui.painter(), spot.center(), Icon::X, 12.0, ink);
        removed = hit.clicked();
    }
    removed
}

// --- Conversation history --------------------------------------------------------------------------

/// One conversation in the history.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConversationEntry {
    pub id: String,
    pub title: String,
    /// When it was last active, in milliseconds since the epoch.
    pub time: i64,
    pub pinned: bool,
}

/// Where a conversation is listed: pinned ones first, then today's, then the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConversationGroup {
    Pinned,
    Today,
    Earlier,
}

/// The start of the day `time` falls in, in UTC.
fn start_of_day(time: i64) -> i64 {
    time.div_euclid(86_400_000) * 86_400_000
}

/// The year, month from 1, day, hours and minutes of a moment in UTC.
fn utc_parts(time: i64) -> (i64, i64, i64, i64, i64) {
    let days = time.div_euclid(86_400_000);
    let rest = time.rem_euclid(86_400_000) / 1000;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day, rest / 3600, rest / 60 % 60)
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Where a conversation is listed, read against `now`: pinned, from today (UTC), or earlier.
pub fn conversation_group(pinned: bool, time: i64, now: i64) -> ConversationGroup {
    if pinned {
        ConversationGroup::Pinned
    } else if time >= start_of_day(now) {
        ConversationGroup::Today
    } else {
        ConversationGroup::Earlier
    }
}

/// The time a conversation shows, read against `now` in UTC: the hour today (`8:35 AM`), the day
/// this year (`Nov 17`), the date before (`Nov 3, 2025`).
pub fn conversation_time(time: i64, now: i64) -> String {
    let (year, month, day, hour, minute) = utc_parts(time);
    if time >= start_of_day(now) {
        let half = if hour < 12 { "AM" } else { "PM" };
        let twelve = if hour % 12 == 0 { 12 } else { hour % 12 };
        return format!("{twelve}:{minute:02} {half}");
    }
    let month = MONTHS[(month - 1) as usize];
    if year == utc_parts(now).0 {
        format!("{month} {day}")
    } else {
        format!("{month} {day}, {year}")
    }
}

/// The words a conversation history says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationHistoryWords {
    pub pinned: String,
    pub today: String,
    pub earlier: String,
    pub rename: String,
    pub pin: String,
    pub unpin: String,
    pub delete: String,
    /// The delete dialog's question: `Delete {title}?`.
    pub delete_question: String,
    pub delete_detail: String,
    pub cancel: String,
    pub empty: String,
}

impl Default for ConversationHistoryWords {
    fn default() -> Self {
        Self {
            pinned: "Pinned".into(),
            today: "Today".into(),
            earlier: "Earlier".into(),
            rename: "Rename".into(),
            pin: "Pin".into(),
            unpin: "Unpin".into(),
            delete: "Delete".into(),
            delete_question: "Delete {title}?".into(),
            delete_detail: "The conversation and everything in it is removed for good.".into(),
            cancel: "Cancel".into(),
            empty: "No conversations yet.".into(),
        }
    }
}

/// What a history was asked this frame, each by the conversation's id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConversationHistoryResponse {
    pub selected: Option<String>,
    /// A title changed in place: the conversation and its new title.
    pub renamed: Option<(String, String)>,
    /// Pinned or unpinned: the conversation and whether it is pinned now.
    pub pinned: Option<(String, bool)>,
    /// Deleted once the dialog asked.
    pub deleted: Option<String>,
}

/// The conversations beside a chat, as sokkel-ui's `ConversationHistory`: newest first under
/// Pinned, Today and Earlier as a list with folding group heads, each row its title and its time at
/// its end, the current one marked; a row's menu (shown on hover) pins or unpins it, renames it in
/// place and deletes it once a dialog has asked. Times and groups are read against `now`.
/// `framed` boxes it; unframed it is a column of its own with a rule at its end. The list's own
/// state is kept in the context under its id.
#[must_use = "show it with `.show(ui)`"]
pub struct ConversationHistory<'a> {
    id: Id,
    conversations: &'a [ConversationEntry],
    current: Option<&'a str>,
    now: i64,
    framed: bool,
    pin: bool,
    rename: bool,
    delete: bool,
    words: ConversationHistoryWords,
}

#[derive(Clone, Default)]
struct HistoryState {
    table: TableState,
    deleting: Option<String>,
}

impl<'a> ConversationHistory<'a> {
    pub fn new(id: impl egui::AsIdSalt, conversations: &'a [ConversationEntry], now: i64) -> Self {
        Self {
            id: Id::unique(id),
            conversations,
            current: None,
            now,
            framed: true,
            pin: true,
            rename: true,
            delete: true,
            words: ConversationHistoryWords::default(),
        }
    }

    /// The conversation shown in the chat, marked.
    pub fn current(mut self, id: Option<&'a str>) -> Self {
        self.current = id;
        self
    }

    pub fn framed(mut self, framed: bool) -> Self {
        self.framed = framed;
        self
    }

    /// Offers Pin and Unpin; on by default.
    pub fn pin(mut self, on: bool) -> Self {
        self.pin = on;
        self
    }

    /// Offers Rename; on by default.
    pub fn rename(mut self, on: bool) -> Self {
        self.rename = on;
        self
    }

    /// Offers Delete; on by default.
    pub fn delete(mut self, on: bool) -> Self {
        self.delete = on;
        self
    }

    pub fn words(mut self, words: ConversationHistoryWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ConversationHistoryResponse {
        let t = tokens(ui.ctx());
        let id = ui.make_persistent_id(self.id);
        let mut held: HistoryState = ui.data(|d| d.get_temp(id)).unwrap_or_default();
        let mut out = ConversationHistoryResponse::default();
        let w = &self.words;
        let mut rows: Vec<&ConversationEntry> = self.conversations.iter().collect();
        rows.sort_by_key(|one| {
            (
                conversation_group(one.pinned, one.time, self.now),
                std::cmp::Reverse(one.time),
            )
        });
        let label = |group: ConversationGroup| match group {
            ConversationGroup::Pinned => w.pinned.clone(),
            ConversationGroup::Today => w.today.clone(),
            ConversationGroup::Earlier => w.earlier.clone(),
        };
        let keys: Vec<String> = rows
            .iter()
            .map(|one| label(conversation_group(one.pinned, one.time, self.now)))
            .collect();
        let cells: Vec<Vec<Cell>> = rows
            .iter()
            .map(|one| {
                vec![
                    if Some(one.id.as_str()) == self.current {
                        Cell::Strong(one.title.clone())
                    } else {
                        Cell::Text(one.title.clone())
                    },
                    Cell::Muted(conversation_time(one.time, self.now)),
                ]
            })
            .collect();
        let time_width = rows
            .iter()
            .map(|one| {
                ui.fonts_mut(|f| {
                    f.layout_no_wrap(
                        conversation_time(one.time, self.now),
                        TextRole::Body.font(&t),
                        Color32::WHITE,
                    )
                    .size()
                    .x
                })
            })
            .fold(0.0_f32, f32::max)
            .ceil();
        let mut title = Column::new("", 80.0).fill().sortable(false);
        if self.rename {
            title = title.editable(Editor::Text);
        }
        let columns = [title, Column::new("", time_width.max(24.0)).sortable(false)];
        let current = rows.iter().position(|one| Some(one.id.as_str()) == self.current);
        held.table.opened = None;
        let menu = |index: usize| -> Vec<MenuItem> {
            let one = rows[index];
            let mut list = Vec::new();
            if self.pin {
                list.push(if one.pinned {
                    MenuItem::action(&w.unpin).icon(Icon::PinOff)
                } else {
                    MenuItem::action(&w.pin).icon(Icon::Pin)
                });
            }
            if self.rename {
                list.push(MenuItem::action(&w.rename).icon(Icon::PencilLine));
            }
            if self.delete {
                if !list.is_empty() {
                    list.push(MenuItem::Separator);
                }
                list.push(MenuItem::action(&w.delete).icon(Icon::Trash2).destructive());
            }
            list
        };
        let offers = self.pin || self.rename || self.delete;
        let mut table = DataTable::new(id.with("table"), &columns, &cells, &mut held.table)
            .variant(TableVariant::List)
            .heads(false)
            .framed(self.framed)
            .footer(false)
            .fits(true)
            .groups(keys)
            .keys(rows.iter().map(|one| one.id.clone()))
            .opened(current)
            .height(f32::INFINITY)
            .words(TableWords {
                empty: w.empty.clone(),
                ..TableWords::default()
            });
        if offers {
            table = table.actions(menu).actions_on_hover(true);
        }
        let shown = table.show(ui);
        if !self.framed {
            let rect = shown.response.rect;
            ui.painter().vline(
                rect.right() - 0.5,
                rect.y_range(),
                Stroke::new(1.0, t.color("border-strong")),
            );
        }
        if let Some(edit) = &shown.edited
            && let Some(one) = rows.get(edit.row)
        {
            let title = edit.value.trim();
            if !title.is_empty() && title != one.title {
                out.renamed = Some((one.id.clone(), title.to_owned()));
            }
        }
        if let Some((index, chosen)) = shown.action
            && let Some(one) = rows.get(index)
        {
            let items = menu(index);
            if let Some(MenuItem::Action { label, .. }) = items.get(chosen) {
                if *label == w.pin || *label == w.unpin {
                    out.pinned = Some((one.id.clone(), !one.pinned));
                } else if *label == w.rename {
                    held.table.edit(index, 0);
                } else if *label == w.delete {
                    held.deleting = Some(one.id.clone());
                }
            }
        }
        if let Some(index) = held.table.opened.take().filter(|index| Some(*index) != current)
            && let Some(one) = rows.get(index)
        {
            out.selected = Some(one.id.clone());
        }
        if let Some(doomed) = held.deleting.clone() {
            let title = self
                .conversations
                .iter()
                .find(|one| one.id == doomed)
                .map_or_else(String::new, |one| one.title.clone());
            let answer = ConfirmDialog::new(
                id.with(("delete", &doomed)),
                w.delete_question.replace("{title}", &title),
                &w.delete,
            )
            .detail(&w.delete_detail)
            .cancel(&w.cancel)
            .destructive(true)
            .show(ui.ctx());
            match answer {
                Some(ConfirmAnswer::Confirm) => {
                    out.deleted = Some(doomed);
                    held.deleting = None;
                }
                Some(_) => held.deleting = None,
                None => {}
            }
        }
        ui.data_mut(|d| d.insert_temp(id, held));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-11-18 09:00 UTC.
    const NOW: i64 = 1_794_992_400_000;

    #[test]
    fn usage_figures_are_written_as_the_web_writes_them() {
        assert_eq!(usage_time(92.0), "92 ms");
        assert_eq!(usage_time(5090.0), "5.09 s");
        assert_eq!(usage_time(12_400.0), "12.4 s");
        assert_eq!(usage_speed(33.42), "33.4");
        assert_eq!(usage_speed(140.6), "141");
        assert_eq!(usage_count(1284.0), "1,284");
        assert_eq!(usage_count(49.0), "49");
    }

    #[test]
    fn conversations_are_grouped_and_timed_against_now() {
        let today = NOW - 25 * 60_000;
        assert_eq!(conversation_group(false, today, NOW), ConversationGroup::Today);
        assert_eq!(conversation_group(true, today, NOW), ConversationGroup::Pinned);
        assert_eq!(
            conversation_group(false, NOW - 86_400_000, NOW),
            ConversationGroup::Earlier
        );
        assert_eq!(conversation_time(today, NOW), "8:35 AM");
        assert_eq!(conversation_time(NOW - 86_400_000, NOW), "Nov 17");
        assert_eq!(conversation_time(NOW - 400 * 86_400_000, NOW), "Oct 14, 2025");
        assert_eq!(conversation_time(start_of_day(NOW) + 13 * 3_600_000, NOW), "1:00 PM");
    }

    #[test]
    fn words_are_counted_as_the_web_counts_them() {
        assert_eq!(word_count("The *short* answer - is 12,50."), 5);
    }
}
