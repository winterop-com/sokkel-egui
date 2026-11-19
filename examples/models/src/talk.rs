//! The chat and the comparison, as the web's `pages/Chat.tsx`, `ChatSettings.tsx` and
//! `Compare.tsx` draw them over each page's own logic in `pages`: the conversations beside the one
//! open, its messages streaming in as the clock moves, the composer, the conversation's settings
//! panel, and one prompt answered by several models side by side.

use eframe::egui::{self, Ui};
use sokkel_egui::AttachmentKind as ComposerKind;
use sokkel_egui::{
    AnswerColumn, AnswerState, AssistantReply, AssistantReplyWords, AssistantState, Bubble, BubbleVariant, Button,
    ButtonSize, ButtonVariant, Columns, Composer, ComposerAttachment, ComposerLayout, ComposerWords,
    Conversation as ConversationPane, ConversationEntry, ConversationHistory, DescriptionList, Empty, Gap, Icon,
    IconButton, Item, ItemSize, ItemVariant, NumberField, Page, PageHeader, PageState, PaneHeader, ReactionChips,
    ReplyFeedback, Section, Select, ShowMore, SidePanel, SliderField, SplitPane, Stack, TextField, TextRole,
    UsageFigures, panel_section, tokens,
};

use crate::data::model::{
    ColumnState, Comparison, Conversation, FORMATS, Feedback, Message, Model, ReplyState, SamplingPatch, Usage,
};
use crate::engine::stream::writing;
use crate::pages::chat_settings;
use crate::pages::{chat, compare};
use crate::screens::Act;
use crate::workbench::{ModelsApp, Views};

/// The library's models as a select, grouped by format, each said as its name and rounding, as the
/// web's `ModelSelect`; quiet, it has no frame of its own. The model chosen, when it changes.
pub fn model_select(ui: &mut Ui, label: &str, models: &[Model], value: &str, quiet: bool) -> Option<String> {
    let t = tokens(ui.ctx());
    // the formats' order, as the select lists them under their names
    let ordered: Vec<&Model> = FORMATS
        .iter()
        .flat_map(|format| models.iter().filter(move |model| model.format == *format))
        .collect();
    let options: Vec<String> = ordered
        .iter()
        .map(|model| format!("{} {}", model.name, model.quant))
        .collect();
    let was = ordered.iter().position(|model| model.id == value);
    let mut chosen = was.unwrap_or(usize::MAX);
    let said = was.map_or("", |at| options[at].as_str());
    let role = if quiet { TextRole::Action } else { TextRole::Body };
    let galley = ui
        .painter()
        .layout_no_wrap(said.to_owned(), role.font(&t), t.color("foreground"));
    // the words, then 6 and the 16 chevron, padded 10 and 8 (quiet: 8 and 6), as the trigger fits them
    let width = galley.size().x + 6.0 + 16.0 + if quiet { 14.0 } else { 18.0 } + 2.0;
    ui.add(
        Select::new(&mut chosen, options, label)
            .small(true)
            .quiet(quiet)
            .width(width.ceil()),
    );
    (Some(chosen) != was && chosen < ordered.len()).then(|| ordered[chosen].id.clone())
}

/// The library's models as a select across its room, as the panel's `ModelSelect className="w-full"`.
pub fn model_select_wide(ui: &mut Ui, label: &str, models: &[Model], value: &str) -> Option<String> {
    let ordered: Vec<&Model> = FORMATS
        .iter()
        .flat_map(|format| models.iter().filter(move |model| model.format == *format))
        .collect();
    let options: Vec<String> = ordered
        .iter()
        .map(|model| format!("{} {}", model.name, model.quant))
        .collect();
    let was = ordered.iter().position(|model| model.id == value);
    let mut chosen = was.unwrap_or(usize::MAX);
    let width = ui.available_width();
    ui.add(Select::new(&mut chosen, options, label).small(true).width(width));
    (Some(chosen) != was && chosen < ordered.len()).then(|| ordered[chosen].id.clone())
}

/// The conversation's model and how it answers, in the panel beside the chat: the model and what
/// it is, temperature and top p, max tokens, the system prompt with its presets, and its defaults.
pub fn settings_panel(ui: &mut Ui, app: &ModelsApp, talk: &Conversation, acts: &mut Vec<Act>) {
    let models = &app.backend.models.seen;
    let model = models.iter().find(|one| one.id == talk.model);
    let said = chat_settings::panel(model, models, &app.backend.loaded.seen);
    let settings = talk.settings.clone();
    let id = talk.id.clone();
    let set = move |patch: SamplingPatch, acts: &mut Vec<Act>| {
        let id = id.clone();
        acts.push(Act::new(move |app| app.backend.set_settings(&id, patch)));
    };
    let mut tab = 0;
    SidePanel::new(("chat-settings", &talk.id), &["Settings"], &mut tab).show(ui, |_, ui| {
        panel_section(ui, "Model", |ui| {
            Stack::new(Gap::Md).show(ui, |ui| {
                if let Some(next) = model_select_wide(ui, "Model", models, &talk.model) {
                    let id = talk.id.clone();
                    acts.push(Act::new(move |app| {
                        app.backend
                            .update(|b| &mut b.conversations, &id, move |row| row.model = next);
                    }));
                }
                match &said.facts {
                    None => {
                        let t = tokens(ui.ctx());
                        ui.label(
                            TextRole::Help
                                .rich(&t, "This model is no longer in the library. Pick another.")
                                .color(t.color("muted-foreground")),
                        );
                    }
                    Some(facts) => {
                        let mut list = DescriptionList::new().side(104.0);
                        for (term, value) in facts {
                            let value = value.clone();
                            list = if *term == "Runner" {
                                list.item(*term, move |ui| crate::screens::chip(ui, &value))
                            } else {
                                list.text(*term, value)
                            };
                        }
                        list.show(ui);
                    }
                }
            });
        });
        let fixed = |value: f64| crate::js::to_fixed(value, 2);
        panel_section(ui, "Sampling", |ui| {
            Stack::new(Gap::Lg).show(ui, |ui| {
                let mut temperature = settings.temperature;
                ui.add(
                    SliderField::new("Temperature", &mut temperature, 2.0)
                        .step(0.05)
                        .format(fixed)
                        .help("Higher is more varied, lower more literal. At 0 asking again gives the same answer."),
                );
                if temperature != settings.temperature {
                    set(
                        SamplingPatch {
                            temperature: Some(temperature),
                            ..SamplingPatch::default()
                        },
                        acts,
                    );
                }
                let mut top_p = settings.top_p;
                ui.add(
                    SliderField::new("Top p", &mut top_p, 1.0)
                        .step(0.05)
                        .format(fixed)
                        .help("Only the likeliest tokens up to this share are drawn from."),
                );
                if top_p != settings.top_p {
                    set(
                        SamplingPatch {
                            top_p: Some(top_p),
                            ..SamplingPatch::default()
                        },
                        acts,
                    );
                }
            });
        });
        panel_section(ui, "Length", |ui| {
            let mut max = settings.max_tokens;
            ui.add(
                NumberField::new(("max-tokens", &talk.id), "Max tokens", &mut max)
                    .integer(true)
                    .step(64.0)
                    .hint(said.max_hint.clone())
                    .help(Some("An answer stops here, its usage line saying so.")),
            );
            if max != settings.max_tokens && chat_settings::max_tokens_kept(Some(max)) {
                set(
                    SamplingPatch {
                        max_tokens: Some(max),
                        ..SamplingPatch::default()
                    },
                    acts,
                );
            }
        });
        panel_section(ui, "System prompt", |ui| {
            Stack::new(Gap::Sm).show(ui, |ui| {
                let key = egui::Id::unique(("chat-system", &talk.id));
                let mut text: String = ui.data(|d| d.get_temp(key)).unwrap_or_else(|| settings.system.clone());
                let field = ui.add(TextField::new("System prompt", &mut text).multiline(true).help(
                    "Said to the model before the conversation. Asking it to be brief keeps answers to a paragraph.",
                ));
                if field.lost_focus() && text != settings.system {
                    set(
                        SamplingPatch {
                            system: Some(text.clone()),
                            ..SamplingPatch::default()
                        },
                        acts,
                    );
                }
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    for preset in chat_settings::PRESETS {
                        let button = Button::new(chat_settings::preset_label(preset))
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Xs);
                        if ui.add(button).clicked() {
                            text = preset.to_owned();
                            set(
                                SamplingPatch {
                                    system: Some(preset.to_owned()),
                                    ..SamplingPatch::default()
                                },
                                acts,
                            );
                        }
                    }
                });
                ui.data_mut(|d| d.insert_temp(key, text));
            });
        });
        if let (Some(model), Some(back)) = (model, &said.back) {
            panel_section(ui, "Defaults", |ui| {
                let button = Button::new(back)
                    .icon(Icon::RotateCcw)
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Sm);
                if ui.add(button).clicked() {
                    set(chat_settings::back_to_defaults(model).1, acts);
                    let key = egui::Id::unique(("chat-system", &talk.id));
                    ui.data_mut(|d| d.remove::<String>(key));
                }
            });
        }
    });
}

/// The words a reply's state says, as the web's `AssistantReply` takes them.
fn assistant_state(state: ReplyState) -> AssistantState {
    match state {
        ReplyState::Streaming => AssistantState::Streaming,
        ReplyState::Done => AssistantState::Done,
        ReplyState::Stopped => AssistantState::Stopped,
        ReplyState::Failed => AssistantState::Failed,
    }
}

/// An answer's usage, as its line says it.
fn figures(usage: &Usage) -> UsageFigures {
    UsageFigures {
        input: Some(usage.input),
        output: usage.output,
        first_token: Some(usage.first_token),
        rate: Some(usage.rate),
        latency: Some(usage.latency),
        note: usage.note.clone(),
    }
}

/// The chat: the conversations beside the one open, its messages, and the composer.
pub fn chat_page(ui: &mut Ui, app: &ModelsApp, id: Option<&str>, views: &mut Views, acts: &mut Vec<Act>) {
    let talks = &app.backend.conversations.seen;
    let models = &app.backend.models.seen;
    let loading = models.is_empty() && app.now() < app.backend.runtime.now + 1;
    let Some(id) = id else {
        // a bare chat address opens the latest conversation, or starts one with the default model
        if !talks.is_empty() || !models.is_empty() {
            let latest = talks.iter().max_by_key(|one| one.time).map(|one| one.id.clone());
            let prefs = app.prefs();
            let model = crate::app::default_model(models, &prefs).cloned();
            acts.push(Act::new(move |app| match (latest, model) {
                (Some(latest), _) => app.go(&format!("chat/{latest}")),
                (None, Some(model)) => app.chat_with(&model),
                _ => {}
            }));
        }
        Page::show(ui, |ui| {
            PageState::loading().show(ui);
        });
        return;
    };
    let Some(talk) = talks.iter().find(|one| one.id == id) else {
        Page::show(ui, |ui| {
            if loading {
                PageState::loading().show(ui);
            } else {
                PageState::empty("This conversation is gone. Pick another, or start a new one.").show(ui);
            }
        });
        return;
    };
    let model = models.iter().find(|one| one.id == talk.model);
    let prefs = app.prefs();
    let start_with = model.or_else(|| crate::app::default_model(models, &prefs)).cloned();
    let busy = writing(&app.backend.live, &format!("chat:{}:", talk.id));
    let now = app.now();
    let show_history = app.kept.chat_history;
    let column = |ui: &mut Ui, acts: &mut Vec<Act>, views: &mut Views| {
        let later = crate::screens::Later::default();
        let mut draft = app.kept.drafts.get(&talk.id).cloned().unwrap_or_default();
        let was = draft.clone();
        ConversationPane::new(("conversation", &talk.id)).show(
            ui,
            |ui| {
                PaneHeader::new()
                    .title_with(|ui| {
                        let label = if show_history {
                            "Hide conversations"
                        } else {
                            "Show conversations"
                        };
                        if ui
                            .add(IconButton::new(Icon::History, label).pressed(show_history))
                            .clicked()
                        {
                            later.push(Act::new(|app| app.kept.chat_history = !app.kept.chat_history));
                        }
                        let t = tokens(ui.ctx());
                        ui.label(
                            TextRole::SectionTitle
                                .rich(&t, &talk.title)
                                .color(t.color("foreground")),
                        );
                    })
                    .show(ui, |ui| {
                        if ui.add(IconButton::new(Icon::MessageSquarePlus, "New chat")).clicked()
                            && let Some(model) = start_with.clone()
                        {
                            later.push(Act::new(move |app| app.chat_with(&model)));
                        }
                    });
            },
            |ui| {
                if talk.messages.is_empty() {
                    start(ui, model, &talk.id, &later);
                    return;
                }
                ui.spacing_mut().item_spacing.y = 10.0;
                for (at, message) in talk.messages.iter().enumerate() {
                    if at > 0 {
                        ui.add_space(14.0);
                    }
                    match message {
                        Message::User(question) => {
                            let t = tokens(ui.ctx());
                            for file in &question.attachments {
                                ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                                    ui.label(
                                        TextRole::Help
                                            .rich(&t, format!("{} \u{b7} {}", file.name, file.detail))
                                            .color(t.color("muted-foreground")),
                                    );
                                });
                            }
                            if !question.text.is_empty() {
                                ui.add(Bubble::new(&question.text).variant(BubbleVariant::Secondary).end(true));
                            }
                            let reactions = question.reactions.clone().unwrap_or_default();
                            if let Some((emoji, name)) = ReactionChips::new(("reactions", &question.id), &reactions)
                                .end(true)
                                .show(ui)
                            {
                                let (talk, message) = (talk.id.clone(), question.id.clone());
                                later.push(Act::new(move |app| app.backend.react(&talk, &message, &emoji, &name)));
                            }
                        }
                        Message::Assistant(answer) => {
                            let said = chat::reply(talk, answer, models, &app.backend.live, now);
                            let version = &answer.versions[answer.at];
                            let words = AssistantReplyWords {
                                thinking: said.thinking.clone(),
                                try_again: "Regenerate".to_owned(),
                                ..AssistantReplyWords::default()
                            };
                            let feedback = version.feedback.map(|one| match one {
                                Feedback::Good => ReplyFeedback::Good,
                                Feedback::Bad => ReplyFeedback::Bad,
                            });
                            let mut reply = AssistantReply::new(("reply", &answer.id), &said.text)
                                .mark(Icon::Bot)
                                .name(&said.name)
                                .code_name(true)
                                .time(crate::data::format::clock(version.time))
                                .state(assistant_state(said.state))
                                .retry(true)
                                .feedback(feedback)
                                .words(words);
                            if let Some(reasoning) = &said.reasoning {
                                reply = reply.reasoning(reasoning);
                            }
                            if let Some(problem) = &said.problem {
                                reply = reply.problem(problem);
                            }
                            if let Some((at, of)) = said.versions {
                                reply = reply.versions(at, of);
                            }
                            let usage = said.usage.clone();
                            let streaming = said.streaming;
                            let out = reply.show(ui, |ui| {
                                if let Some(usage) = &usage {
                                    ui.add(figures(usage).line().live(streaming));
                                }
                            });
                            let (talk_id, message) = (talk.id.clone(), answer.id.clone());
                            if out.copied {
                                ui.ctx().copy_text(said.text.clone());
                            }
                            if out.retry {
                                let (talk_id, message) = (talk_id.clone(), message.clone());
                                later.push(Act::new(move |app| {
                                    app.backend.regenerate(&talk_id, &message);
                                }));
                            }
                            if let Some(feedback) = out.feedback {
                                let feedback = feedback.map(|one| match one {
                                    ReplyFeedback::Good => Feedback::Good,
                                    ReplyFeedback::Bad => Feedback::Bad,
                                });
                                let (talk_id, message) = (talk_id.clone(), message.clone());
                                later.push(Act::new(move |app| {
                                    app.backend.give_feedback(&talk_id, &message, feedback)
                                }));
                            }
                            if let Some(at) = out.version {
                                later.push(Act::new(move |app| {
                                    app.backend.show_version(&talk_id, &message, at.saturating_sub(1))
                                }));
                            }
                        }
                    }
                }
            },
            |ui| {
                let attachments: Vec<ComposerAttachment> = views
                    .files
                    .iter()
                    .map(|file| ComposerAttachment {
                        id: file.id.clone(),
                        name: file.name.clone(),
                        kind: match file.kind {
                            crate::data::model::AttachmentKind::Image => ComposerKind::Image,
                            crate::data::model::AttachmentKind::File => ComposerKind::File,
                        },
                        detail: Some(file.detail.clone()),
                        uploading: false,
                    })
                    .collect();
                let mut picked = None;
                let out = Composer::new(("composer", &talk.id), &mut draft)
                    .layout(ComposerLayout::Inline)
                    .busy(busy)
                    .attach(true)
                    .attachments(&attachments)
                    .words(ComposerWords {
                        placeholder: chat::placeholder(model),
                        ..ComposerWords::default()
                    })
                    .show(ui, |ui| {
                        picked = model_select(ui, "Model", models, &talk.model, true);
                    });
                if let Some(next) = picked {
                    let id = talk.id.clone();
                    later.push(Act::new(move |app| {
                        app.backend
                            .update(|b| &mut b.conversations, &id, move |row| row.model = next);
                    }));
                }
                if let Some(gone) = out.removed {
                    views.files.retain(|one| one.id != gone);
                }
                if out.stop {
                    let id = talk.id.clone();
                    later.push(Act::new(move |app| app.backend.stop_conversation(&id)));
                }
                if out.send && !busy {
                    let (id, text, files) = (talk.id.clone(), draft.clone(), std::mem::take(&mut views.files));
                    later.push(Act::new(move |app| {
                        app.backend.send(&id, &text, files);
                    }));
                    draft.clear();
                }
            },
        );
        if draft != was {
            let id = talk.id.clone();
            later.push(Act::new(move |app| {
                app.kept.drafts.insert(id, draft);
            }));
        }
        acts.extend(later.take());
    };
    if show_history {
        let entries: Vec<ConversationEntry> = talks
            .iter()
            .map(|one| ConversationEntry {
                id: one.id.clone(),
                title: one.title.clone(),
                time: one.time,
                pinned: one.pinned,
            })
            .collect();
        let mut history = None;
        let mut column_acts = Vec::new();
        SplitPane::new("models.chat.split")
            .default_share(0.26)
            .min_share(0.18)
            .max_share(0.4)
            .show(
                ui,
                |ui| {
                    history = Some(
                        ConversationHistory::new("chat-history", &entries, now)
                            .current(Some(&talk.id))
                            .framed(false)
                            .show(ui),
                    );
                },
                |ui| column(ui, &mut column_acts, views),
            );
        acts.extend(column_acts);
        if let Some(out) = history {
            if let Some(id) = out.selected {
                acts.push(Act::go(format!("chat/{id}")));
            }
            if let Some((id, title)) = out.renamed {
                acts.push(Act::new(move |app| {
                    app.backend
                        .update(|b| &mut b.conversations, &id, move |row| row.title = title);
                }));
            }
            if let Some((id, pinned)) = out.pinned {
                acts.push(Act::new(move |app| {
                    app.backend
                        .update(|b| &mut b.conversations, &id, move |row| row.pinned = pinned);
                }));
            }
            if let Some(id) = out.deleted {
                let current = talk.id.clone();
                acts.push(Act::new(move |app| {
                    app.backend.stop_conversation(&id);
                    let promise = app.backend.remove(|b| &mut b.conversations, &id);
                    if id == current {
                        app.then(promise, |app, _| app.go("chat"));
                    }
                }));
            }
        }
    } else {
        column(ui, acts, views);
    }
}

/// A conversation not yet begun: the model it talks to, and four things to ask.
fn start(ui: &mut Ui, model: Option<&Model>, talk: &str, acts: &crate::screens::Later) {
    let title = format!("Ask {}", model.map_or("a model", |one| one.name.as_str()));
    Empty::new(title)
        .icon(Icon::Bot)
        .description("It runs on this machine; nothing you write leaves it.")
        .show(ui, |ui| {
            Columns::new(2)
                .min(200.0)
                .gap(8.0)
                .show(ui, chat::SUGGESTIONS.len(), |ui, index, _| {
                    let (title, prompt) = chat::SUGGESTIONS[index];
                    let pressed = Item::new(title)
                        .description(prompt)
                        .variant(ItemVariant::Outline)
                        .size(ItemSize::Xs)
                        .pressable(true)
                        .show(ui, |_| {});
                    if pressed.clicked() {
                        let talk = talk.to_owned();
                        acts.push(Act::new(move |app| {
                            app.backend.send(&talk, prompt, Vec::new());
                        }));
                    }
                });
        });
}

/// One prompt sent to two, three or four models side by side, each column streaming its answer
/// with its usage under it.
pub fn compare_page(ui: &mut Ui, app: &ModelsApp, id: Option<&str>, views: &mut Views, acts: &mut Vec<Act>) {
    let _ = views;
    let comparisons = &app.backend.comparisons.seen;
    let models = &app.backend.models.seen;
    let draft = app.kept.compare.clone();
    let now = app.now();
    let said = compare::compare_page(comparisons, models, &app.backend.live, &draft, id, now);
    let shown = said
        .shown
        .as_ref()
        .and_then(|id| comparisons.iter().find(|one| one.id == *id))
        .cloned();
    let change = |acts: &mut Vec<Act>, next: compare::Draft| {
        acts.push(Act::new(move |app| app.kept.compare = next));
    };
    Page::scroll(ui, |ui| {
        let mut header = PageHeader::new("Compare");
        if let Some(count) = &said.count {
            header = header.count(count);
        }
        header.show(ui, |ui| {
            if let Some(one) = &shown
                && ui
                    .add(IconButton::new(Icon::Trash2, "Delete this comparison"))
                    .clicked()
            {
                let id = one.id.clone();
                acts.push(Act::new(move |app| {
                    app.backend.stop_comparison(&id);
                    let promise = app.backend.remove(|b| &mut b.comparisons, &id);
                    app.then(promise, |app, _| app.go("compare/new"));
                }));
            }
            if !said.earlier.is_empty() {
                let labels: Vec<String> = said.earlier.iter().map(|(_, when)| when.clone()).collect();
                let was = shown
                    .as_ref()
                    .and_then(|one| said.earlier.iter().position(|(id, _)| *id == one.id));
                let mut at = was.unwrap_or(usize::MAX);
                let t = tokens(ui.ctx());
                let longest = labels
                    .iter()
                    .map(|one| {
                        ui.painter()
                            .layout_no_wrap(one.clone(), TextRole::Body.font(&t), t.color("foreground"))
                            .size()
                            .x
                    })
                    .fold(0.0, f32::max);
                ui.add(
                    Select::new(&mut at, labels, "Earlier comparisons")
                        .placeholder("Earlier")
                        .small(true)
                        .width((longest + 40.0).min(224.0).ceil()),
                );
                if Some(at) != was
                    && let Some((id, _)) = said.earlier.get(at)
                {
                    acts.push(Act::go(format!("compare/{id}")));
                }
            }
        });
        Stack::new(Gap::Page).show(ui, |ui| {
            Section::new("Prompt")
                .description(
                    "Sent to every model chosen, each answering on its own at the same settings, in columns in the order the models stand here.",
                )
                .show(ui, |ui| {
                    Stack::new(Gap::Lg).show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                            for (index, model) in draft.models.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 2.0;
                                    let t = tokens(ui.ctx());
                                    let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 28.0), egui::Sense::drag());
                                    sokkel_egui::icons::paint(
                                        ui.painter(),
                                        rect.center(),
                                        Icon::GripVertical,
                                        16.0,
                                        t.color("muted-foreground"),
                                    );
                                    let label = format!("Model {}", index + 1);
                                    if let Some(next) = model_select(ui, &label, models, model, false) {
                                        let mut changed = draft.clone();
                                        changed.models[index] = next;
                                        change(acts, changed);
                                    }
                                    if said.can_remove {
                                        let label = format!("Take model {} away", index + 1);
                                        if ui.add(IconButton::new(Icon::X, label)).clicked() {
                                            let mut changed = draft.clone();
                                            changed.models.remove(index);
                                            change(acts, changed);
                                        }
                                    }
                                });
                            }
                            if said.can_add {
                                let add = Button::new("Add a model")
                                    .icon(Icon::Plus)
                                    .variant(ButtonVariant::Outline)
                                    .size(ButtonSize::Sm);
                                if ui.add(add).clicked() {
                                    change(acts, compare::add_model(&draft, models));
                                }
                            }
                        });
                        Columns::new(2).min(320.0).gap(24.0).show(ui, 2, |ui, index, _| {
                            if index == 0 {
                                let mut temperature = draft.temperature;
                                ui.add(
                                    SliderField::new("Temperature", &mut temperature, 2.0)
                                        .step(0.05)
                                        .format(|value| crate::js::to_fixed(value, 2)),
                                );
                                if temperature != draft.temperature {
                                    change(
                                        acts,
                                        compare::Draft {
                                            temperature,
                                            ..draft.clone()
                                        },
                                    );
                                }
                            } else {
                                let mut max = draft.max_tokens;
                                ui.add(
                                    NumberField::new("compare-max-tokens", "Max tokens", &mut max)
                                        .integer(true)
                                        .step(64.0),
                                );
                                if max != draft.max_tokens && max >= 1.0 {
                                    change(
                                        acts,
                                        compare::Draft {
                                            max_tokens: max.min(8192.0),
                                            ..draft.clone()
                                        },
                                    );
                                }
                            }
                        });
                        let mut prompt = draft.prompt.clone();
                        let out = Composer::new("compare-composer", &mut prompt)
                            .busy(said.running)
                            .disabled(said.disabled)
                            .words(ComposerWords {
                                placeholder: said.placeholder.clone(),
                                send: "Run".to_owned(),
                                ..ComposerWords::default()
                            })
                            .show(ui, |_| {});
                        if out.stop
                            && let Some(one) = &shown
                        {
                            let id = one.id.clone();
                            acts.push(Act::new(move |app| app.backend.stop_comparison(&id)));
                        }
                        if out.send && said.ready {
                            let picked: Vec<String> = draft
                                .models
                                .iter()
                                .filter(|model| models.iter().any(|one| one.id == **model))
                                .cloned()
                                .collect();
                            let asked = draft.clone();
                            acts.push(Act::new(move |app| {
                                let promise = app.backend.compare(
                                    crate::js::trim(&asked.prompt),
                                    &picked,
                                    asked.temperature,
                                    asked.max_tokens,
                                );
                                app.then(promise, |app, outcome| {
                                    if let Ok(made) = outcome
                                        && let Some(made) = made.downcast_ref::<Comparison>()
                                    {
                                        let route = format!("compare/{}", made.id);
                                        app.go(&route);
                                    }
                                });
                            }));
                        } else if prompt != draft.prompt {
                            change(
                                acts,
                                compare::Draft {
                                    prompt,
                                    ..draft.clone()
                                },
                            );
                        }
                    });
                });
            let Some(one) = &shown else {
                Empty::new("Nothing compared yet")
                    .icon(Icon::Columns3)
                    .description("Choose the models, write a prompt, and press Enter: each answers in a column of its own.")
                    .show(ui, |_| {});
                return;
            };
            Stack::new(Gap::Lg).show(ui, |ui| {
                Section::new("Asked")
                    .description(said.asked.clone().unwrap_or_default())
                    .actions(|ui| {
                        if said.running {
                            let stop = Button::new("Stop all")
                                .icon(Icon::Square)
                                .variant(ButtonVariant::Outline)
                                .size(ButtonSize::Sm);
                            if ui.add(stop).clicked() {
                                let id = one.id.clone();
                                acts.push(Act::new(move |app| app.backend.stop_comparison(&id)));
                            }
                        } else {
                            let again = Button::new("Run all again")
                                .icon(Icon::RotateCcw)
                                .variant(ButtonVariant::Outline)
                                .size(ButtonSize::Sm);
                            if ui.add(again).clicked() {
                                let (id, count) = (one.id.clone(), one.columns.len());
                                acts.push(Act::new(move |app| {
                                    for index in 0..count {
                                        app.backend.rerun_column(&id, index);
                                    }
                                }));
                            }
                        }
                        let edit = Button::new("Edit")
                            .icon(Icon::Pencil)
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Sm);
                        if ui.add(edit).clicked() {
                            change(
                                acts,
                                compare::Draft {
                                    prompt: one.prompt.clone(),
                                    models: one.columns.iter().map(|column| column.model.clone()).collect(),
                                    temperature: one.temperature,
                                    max_tokens: one.max_tokens,
                                },
                            );
                        }
                    })
                    .show(ui, |ui| {
                        ShowMore::new(("asked", &one.id), &one.prompt).show(ui);
                    });
                let count = said.columns;
                Columns::new(count).min(320.0).gap(24.0).show(ui, said.answers.len(), |ui, index, _| {
                    let answer = &said.answers[index];
                    let state = match answer.state {
                        ColumnState::Waiting => AnswerState::Waiting,
                        ColumnState::Streaming => AnswerState::Streaming,
                        ColumnState::Done => AnswerState::Done,
                        ColumnState::Stopped => AnswerState::Stopped,
                        ColumnState::Failed => AnswerState::Failed,
                    };
                    let mut column = AnswerColumn::new(("answer", &one.id, index), &answer.model, state, &answer.content)
                        .detail(&answer.detail)
                        .stop(true)
                        .retry(true)
                        .remove(answer.removable);
                    if let Some(reasoning) = &answer.reasoning {
                        column = column.reasoning(reasoning);
                    }
                    if let Some(usage) = &answer.usage {
                        column = column.usage(figures(usage));
                    }
                    if let Some(problem) = &answer.problem {
                        column = column.problem(problem);
                    }
                    let out = column.show(ui);
                    let id = one.id.clone();
                    if out.copied {
                        ui.ctx().copy_text(answer.content.clone());
                    }
                    if out.stop {
                        let id = id.clone();
                        acts.push(Act::new(move |app| app.backend.stop_column(&id, index)));
                    }
                    if out.retry {
                        let id = id.clone();
                        acts.push(Act::new(move |app| {
                            app.backend.rerun_column(&id, index);
                        }));
                    }
                    if out.remove {
                        acts.push(Act::new(move |app| {
                            app.backend.stop_column(&id, index);
                            app.backend.update(|b| &mut b.comparisons, &id, move |row| {
                                row.columns.remove(index);
                            });
                        }));
                    }
                });
            });
        });
    });
}
