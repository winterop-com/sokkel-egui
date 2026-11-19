//! A dialog that asks for one value, after sokkel-ui's `PromptDialog` (the contract's
//! `patterns/prompt`): a dialog at `sm` with one field, Cancel and Submit.

use egui::{Context, Id, Key, Ui};

use super::overlay::Dialog;
use crate::{Button, ButtonVariant, tokens};

/// The words a prompt says.
#[derive(Clone, Debug)]
pub struct PromptWords {
    /// The confirm; an app names it for what it does (Create, Save).
    pub submit: String,
    /// The confirm while the value is being taken.
    pub submitting: String,
    pub cancel: String,
    /// Why Submit is shut while the field is empty.
    pub empty: String,
    /// What the status slot says when taking the value failed with no words of its own.
    pub failed: String,
    /// Cancel's word once a busy submit has run `busy-release`.
    pub stop_waiting: String,
}

impl Default for PromptWords {
    fn default() -> Self {
        Self {
            submit: "Submit".into(),
            submitting: "Submitting".into(),
            cancel: "Cancel".into(),
            empty: "Fill in the field first.".into(),
            failed: "Could not submit. Try again.".into(),
            stop_waiting: "Stop waiting".into(),
        }
    }
}

/// How a prompt was answered this frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromptAnswer {
    /// The value to take. The app takes it at once and closes the prompt, or holds it `busy`
    /// while its work runs and says a failure with `error` (the prompt stays open, the value kept).
    Submit(String),
    /// Cancel, Escape or a press on the shade: nothing taken.
    Cancel,
    /// Stop waiting, or Escape, once a busy submit has run `busy-release`: the app lets the work go.
    StopWaiting,
}

/// A prompt's check on its value: why it is refused, or nothing.
pub type PromptCheck = dyn Fn(&str) -> Option<String>;

/// What a prompt keeps between frames: the value, whether its issue shows, and the frame it was
/// last drawn on, so an opening is told from the frames that follow it.
#[derive(Clone, Debug, Default)]
struct Held {
    value: String,
    shown: bool,
    frame: u64,
    /// Enter submitted nothing: the field takes the focus back.
    refocus: bool,
}

/// A dialog that asks for one value: its title what is asked, an optional line under it, one field
/// (a text field, or a [`crate::SecretField`] with `secret`), and Cancel and Submit. Enter in the
/// field submits. Submit is shut while the field is empty (unless `allow_empty`) and while
/// `validate` refuses the value, whose issue the field shows once something is typed or Submit is
/// pressed. While the app takes the value it holds the prompt `busy`: Submit spins, the field is
/// read only, and Cancel, Escape and the shade wait until `busy-release`. When taking it fails the
/// app says why with `error`, in the status slot over the footer, the value kept. Every opening
/// starts over from `default_value`.
pub struct PromptDialog<'a> {
    id: Id,
    title: String,
    description: Option<String>,
    label: String,
    hint: Option<String>,
    help: Option<String>,
    placeholder: Option<String>,
    default_value: String,
    secret: bool,
    mono: bool,
    allow_empty: bool,
    validate: Option<&'a PromptCheck>,
    busy: bool,
    error: Option<String>,
    failed: bool,
    busy_release: Option<f32>,
    words: PromptWords,
}

impl<'a> PromptDialog<'a> {
    /// A prompt asking `title`, its field labelled `label`.
    pub fn new(id: impl egui::AsIdSalt, title: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: Id::unique(id),
            title: title.into(),
            description: None,
            label: label.into(),
            hint: None,
            help: None,
            placeholder: None,
            default_value: String::new(),
            secret: false,
            mono: false,
            allow_empty: false,
            validate: None,
            busy: false,
            error: None,
            failed: false,
            busy_release: None,
            words: PromptWords::default(),
        }
    }

    /// One line under the title on what the value is for.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// A short note beside the label: a unit, a format.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// A sentence under the input.
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// What the field holds each time the prompt opens: a rename starts from the name.
    pub fn default_value(mut self, value: impl Into<String>) -> Self {
        self.default_value = value.into();
        self
    }

    /// The value is a secret: a secret field, masked with its reveal, in the code face.
    pub fn secret(mut self, secret: bool) -> Self {
        self.secret = secret;
        self
    }

    /// The value in the code face, for codes, keys and addresses.
    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    /// An empty value may be submitted.
    pub fn allow_empty(mut self, allow: bool) -> Self {
        self.allow_empty = allow;
        self
    }

    /// Says why a value is refused, or nothing.
    pub fn validate(mut self, validate: &'a PromptCheck) -> Self {
        self.validate = Some(validate);
        self
    }

    /// The value is being taken: held from the app while its work runs.
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    /// Taking the value failed, saying why: the prompt stays open, the value kept.
    pub fn error(mut self, error: Option<impl Into<String>>) -> Self {
        self.error = error.map(Into::into);
        self
    }

    /// Taking the value failed with no words of its own: the status slot says `words.failed`.
    pub fn failed(mut self, failed: bool) -> Self {
        self.failed = failed;
        self
    }

    /// How long, in milliseconds, a busy submit holds the prompt; `busy-release` by default.
    pub fn busy_release(mut self, ms: f32) -> Self {
        self.busy_release = Some(ms);
        self
    }

    pub fn words(mut self, words: PromptWords) -> Self {
        self.words = words;
        self
    }

    /// Show the prompt; `Some` once it is answered.
    pub fn show(self, ctx: &Context) -> Option<PromptAnswer> {
        let held_id = self.id.with("held");
        let frame = ctx.cumulative_frame_nr();
        let mut held: Held = ctx.data(|d| d.get_temp(held_id)).unwrap_or_default();
        // an opening: not drawn the frame before, so it starts over
        let opening = held.frame + 1 < frame || held.frame == 0;
        if opening {
            held = Held {
                value: self.default_value.clone(),
                shown: false,
                frame,
                refocus: false,
            };
        }
        held.frame = frame;
        let busy = self.busy;
        // how long the submit has been busy, to let the prompt go after `busy-release`
        let since = self.id.with("busy-since");
        let now = ctx.input(|i| i.time);
        let released = if busy {
            let started = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(since, || now));
            let wait = f64::from(
                self.busy_release
                    .or_else(|| tokens(ctx).duration("busy-release"))
                    .unwrap_or(5000.0),
            ) / 1000.0;
            let left = started + wait - now;
            if left > 0.0 {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(left));
            }
            left <= 0.0
        } else {
            ctx.data_mut(|d| d.remove_temp::<f64>(since));
            false
        };
        let issue = self.validate.and_then(|validate| validate(&held.value));
        let empty = !self.allow_empty && held.value.trim().is_empty();
        // Submit shut while the field is empty or refused, with its reason
        let why = if empty {
            Some(self.words.empty.clone())
        } else {
            issue.clone()
        };
        let status = self
            .error
            .clone()
            .or_else(|| self.failed.then(|| self.words.failed.clone()));
        let mut dialog = Dialog::new(self.id, self.title.clone()).width(384.0).status(status);
        dialog.sheet = false;
        if let Some(description) = &self.description {
            dialog = dialog.description(description.clone());
        }
        let mut answer = None;
        let mut enter = false;
        let mut pressed_shut = false;
        let shown_issue = (held.shown || !held.value.is_empty()).then(|| issue.clone()).flatten();
        let focus = opening || std::mem::take(&mut held.refocus);
        let held = std::cell::RefCell::new(held);
        let closed = dialog.draw(
            ctx,
            Some(|ui: &mut Ui| {
                let field = if self.secret {
                    let mut held = held.borrow_mut();
                    let mut field =
                        crate::SecretField::new(self.id.with("secret"), self.label.clone(), &mut held.value)
                            .read_only(busy)
                            .focus(focus)
                            .error(shown_issue.clone());
                    if let Some(hint) = &self.hint {
                        field = field.hint(hint.clone());
                    }
                    if let Some(help) = &self.help {
                        field = field.help(help.clone());
                    }
                    if let Some(placeholder) = &self.placeholder {
                        field = field.placeholder(placeholder.clone());
                    }
                    ui.add(field)
                } else {
                    let label = self.label.clone();
                    super::Field::new(self.label.clone())
                        .hint(self.hint.clone())
                        .help_if(self.help.clone())
                        .error(shown_issue.clone())
                        .show(ui, |ui| {
                            let mut held = held.borrow_mut();
                            let before = held.value.clone();
                            let mut input = super::Input::new(&mut held.value)
                                .mono(self.mono)
                                .read_only(busy)
                                .invalid(shown_issue.is_some())
                                .name(label);
                            if let Some(placeholder) = &self.placeholder {
                                input = input.placeholder(placeholder.clone());
                            }
                            let response = ui.add(input);
                            if busy {
                                held.value = before;
                            }
                            if focus {
                                response.request_focus();
                            }
                            response
                        })
                };
                // Enter in the field submits, as a press on Submit does
                if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    enter = true;
                }
            }),
            |ui: &mut Ui| {
                let label = if busy {
                    self.words.submitting.clone()
                } else {
                    self.words.submit.clone()
                };
                let mut submit = Button::new(label).busy(busy);
                if let Some(why) = &why
                    && !busy
                {
                    submit = submit.reason(why.clone());
                }
                let response = ui.add(submit);
                if response.clicked() && why.is_none() && !busy {
                    answer = Some(PromptAnswer::Submit(held.borrow().value.clone()));
                }
                // a press on a shut Submit shows the issue at once
                if why.is_some()
                    && ui.input(|i| i.pointer.primary_clicked())
                    && ui
                        .input(|i| i.pointer.interact_pos())
                        .is_some_and(|p| response.rect.contains(p))
                {
                    pressed_shut = true;
                }
                let words = if released {
                    &self.words.stop_waiting
                } else {
                    &self.words.cancel
                };
                let cancel = Button::new(words.clone()).variant(ButtonVariant::Ghost);
                if ui.add_enabled(!busy || released, cancel).clicked() {
                    answer = Some(if released {
                        PromptAnswer::StopWaiting
                    } else {
                        PromptAnswer::Cancel
                    });
                }
            },
        );
        let mut held = held.into_inner();
        if enter && !busy {
            if why.is_none() {
                answer = answer.or(Some(PromptAnswer::Submit(held.value.clone())));
            } else {
                pressed_shut = true;
            }
        }
        if enter && answer.is_none() {
            // the field keeps the focus after an Enter that submits nothing
            held.refocus = true;
        }
        if pressed_shut {
            held.shown = true;
        }
        if released && ctx.input(|i| i.key_pressed(Key::Escape)) {
            answer = answer.or(Some(PromptAnswer::StopWaiting));
        }
        let answer = answer.or((closed && !busy).then_some(PromptAnswer::Cancel));
        ctx.data_mut(|d| d.insert_temp(held_id, held));
        answer
    }
}
