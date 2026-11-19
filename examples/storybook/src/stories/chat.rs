//! The chat parts: the usage line, the question's bubble with its reactions, the assistant's reply,
//! an answer column beside others, the composer, the conversations beside a chat, and a whole
//! conversation, with Tidevann's models answering about its jobs.

use eframe::egui::{Ui, vec2};
use sokkel_egui::{
    AnswerColumn, AnswerState, AssistantReply, AssistantState, AttachmentKind, Bubble, BubbleVariant, Button,
    ButtonSize, ButtonVariant, Composer, ComposerAttachment, ComposerLayout, ComposerWords, Conversation,
    ConversationEntry, ConversationHistory, Icon, IconButton, PaneHeader, Reaction, ReactionChips, ReplyFeedback,
    UsageFigures, UsageLine,
};

use super::{State, section};

/// 2026-11-18 09:00 UTC, the moment the stories read times against.
const NOW: i64 = 1_794_992_400_000;
const MINUTE: i64 = 60_000;
const DAY: i64 = 86_400_000;

const JQ_ANSWER: &str = "A jq filter that keeps the failed runs and says which step each stopped at:\n\n```jq\n.runs[]\n| select(.state == \"failed\")\n| {job, step: .at.step, started}\n```\n\nRun it over the API's answer:\n\n```sh\ncurl -s https://tidevann.example.org/api/runs | jq -f failed.jq\n```";

const SHORT_ANSWER: &str = "`nightly-export` stopped at `validate`: **37 of 2,184 rows** had an `amount` that was not a number, mostly decimal commas like `12,50`.\n\n- `transform` now parses amounts with the source's locale.\n- Blank amounts are dropped before `validate`.\n\nThe next run is tonight at 02:00. Questions go to ada.";

const REASONING: &str = "The export failed at validate. The rows that failed have amounts with a comma. The feed changed on October 14, so the parser reads 12,50 as text. I should say what changed and where the fix belongs.";

/// A text the story keeps between frames, under a key of its own.
fn kept(ui: &Ui, key: &str, first: &str) -> String {
    ui.data(|d| d.get_temp::<String>(eframe::egui::Id::unique(("story-chat", key))))
        .unwrap_or_else(|| first.to_owned())
}

fn keep(ui: &Ui, key: &str, text: String) {
    ui.data_mut(|d| d.insert_temp(eframe::egui::Id::unique(("story-chat", key)), text));
}

fn usage_of(input: f64, output: f64, first: f64, rate: f64, latency: f64) -> UsageFigures {
    UsageFigures {
        input: Some(input),
        output,
        first_token: Some(first),
        rate: Some(rate),
        latency: Some(latency),
        note: None,
    }
}

pub fn usage_line(_: &mut State, ui: &mut Ui) {
    section(
        ui,
        "An answer that ended, one still coming, and one stopped at its limit",
    );
    ui.add(
        UsageLine::new(167.0)
            .input(49.0)
            .first_token(92.0)
            .rate(33.4)
            .latency(5090.0),
    );
    ui.add_space(12.0);
    ui.add(UsageLine::new(48.0).input(1284.0).first_token(230.0).live(true));
    ui.add_space(12.0);
    ui.add(
        UsageLine::new(512.0)
            .input(257.0)
            .first_token(410.0)
            .rate(142.6)
            .latency(12_400.0)
            .note("max tokens"),
    );
}

pub fn bubble(_: &mut State, ui: &mut Ui) {
    let width = 720.0_f32.min(ui.available_width());
    section(
        ui,
        "A question at the row's end, its reactions and the add button under it",
    );
    ui.allocate_ui(vec2(width, 200.0), |ui| {
        ui.set_width(width);
        ui.spacing_mut().item_spacing.y = 10.0;
        ui.add(
            Bubble::new("Now a jq query for the failed runs in the API answer.")
                .variant(BubbleVariant::Secondary)
                .end(true),
        );
        let reactions = [Reaction {
            id: "\u{1F44D}".into(),
            label: "thumbs up".into(),
            count: 2,
            mine: true,
        }];
        let _ = ReactionChips::new("story-reactions", &reactions).end(true).show(ui);
        ui.add_space(14.0);
        ui.add(Bubble::new("Why did nightly-export fail last night?").variant(BubbleVariant::Default));
        let _ = ReactionChips::new("story-reactions-empty", &[]).show(ui);
        ui.add(
            Bubble::new("Summarize this week for the harbor project: failures, timeouts and anything slow.")
                .variant(BubbleVariant::Muted),
        );
    });
}

pub fn assistant_reply(_: &mut State, ui: &mut Ui) {
    let width = 736.0_f32.min(ui.available_width());
    ui.allocate_ui(vec2(width, 860.0), |ui| {
        ui.set_width(width);
        section(ui, "Written, asked for twice, with its usage and feedback");
        let _ = AssistantReply::new("story-reply", JQ_ANSWER)
            .mark(Icon::Bot)
            .name("brine-coder-14b")
            .code_name(true)
            .time("08:32")
            .retry(true)
            .feedback(Some(ReplyFeedback::Good))
            .versions(2, 2)
            .show(ui, |ui| {
                ui.add(
                    UsageLine::new(72.0)
                        .input(257.0)
                        .first_token(230.0)
                        .rate(30.4)
                        .latency(2600.0),
                );
            });
        ui.add_space(16.0);
        section(ui, "Streaming with nothing written yet, and failed with Try again");
        let half = ((width - 16.0) / 2.0).floor();
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            ui.allocate_ui(vec2(half, 300.0), |ui| {
                ui.set_width(half);
                let _ = AssistantReply::new("story-thinking", "")
                    .mark(Icon::Bot)
                    .name("tide-7b-instruct")
                    .code_name(true)
                    .time("08:40")
                    .state(AssistantState::Streaming)
                    .show(ui, |_| {});
            });
            ui.allocate_ui(vec2(half, 300.0), |ui| {
                ui.set_width(half);
                let _ = AssistantReply::new("story-failed", "")
                    .mark(Icon::Bot)
                    .name("reef-32b")
                    .code_name(true)
                    .time("08:41")
                    .state(AssistantState::Failed)
                    .problem("The runner stopped before the answer was finished.")
                    .retry(true)
                    .show(ui, |_| {});
            });
        });
    });
}

pub fn answer_column(_: &mut State, ui: &mut Ui) {
    section(
        ui,
        "Three models side by side: done, thinking then answering, and failed",
    );
    let gap = 24.0;
    let width = ((ui.available_width() - 2.0 * gap) / 3.0).floor();
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.allocate_ui(vec2(width, 600.0), |ui| {
            ui.set_width(width);
            let _ = AnswerColumn::new("story-kelp", "kelp-3b", AnswerState::Done, SHORT_ANSWER)
                .detail("Q4_K_M · 1.90 GB")
                .usage(usage_of(31.0, 74.0, 120.0, 41.2, 1920.0))
                .retry(true)
                .remove(true)
                .show(ui);
        });
        ui.allocate_ui(vec2(width, 600.0), |ui| {
            ui.set_width(width);
            let _ = AnswerColumn::new(
                "story-reef",
                "reef-8b",
                AnswerState::Streaming,
                "The short answer is that the input changed and the check did not.",
            )
            .detail("Q4_K_M · 4.87 GB")
            .reasoning(REASONING)
            .usage(UsageFigures {
                input: Some(31.0),
                output: 14.0,
                first_token: Some(640.0),
                ..UsageFigures::default()
            })
            .stop(true)
            .remove(true)
            .show(ui);
        });
        ui.allocate_ui(vec2(width, 600.0), |ui| {
            ui.set_width(width);
            let _ = AnswerColumn::new("story-swell", "swell-70b", AnswerState::Failed, "")
                .detail("Q4_K_M · 42.5 GB")
                .problem("It needs 44.1 GB of memory and the runner may use 24.0 GB.")
                .retry(true)
                .remove(true)
                .show(ui);
        });
    });
}

fn model_tool(ui: &mut Ui) {
    let _ = ui.add(
        Button::new("brine-coder-14b 4bit")
            .trailing(Icon::ChevronDown)
            .variant(ButtonVariant::Ghost)
            .size(ButtonSize::Sm),
    );
}

pub fn composer(_: &mut State, ui: &mut Ui) {
    let width = 720.0_f32.min(ui.available_width());
    ui.allocate_ui(vec2(width, 860.0), |ui| {
        ui.set_width(width);
        section(ui, "Inline: one line, a pill, with a model's select in its tools");
        let mut text = kept(ui, "inline", "");
        let _ = Composer::new("story-inline", &mut text)
            .layout(ComposerLayout::Inline)
            .attach(true)
            .words(ComposerWords {
                placeholder: "Ask brine-coder-14b".into(),
                ..ComposerWords::default()
            })
            .show(ui, model_tool);
        keep(ui, "inline", text);
        ui.add_space(20.0);
        section(ui, "Inline, opened into the box by a long question, while an answer streams");
        let mut long = kept(
            ui,
            "long",
            "Explain why nightly-export failed last night, which rows it stopped at, what changed in the supplier feed on October 14, and where the fix belongs.",
        );
        let _ = Composer::new("story-inline-open", &mut long)
            .layout(ComposerLayout::Inline)
            .attach(true)
            .busy(true)
            .show(ui, model_tool);
        keep(ui, "long", long);
        ui.add_space(20.0);
        section(ui, "Stacked, with files waiting to go");
        let files = [
            ComposerAttachment {
                id: "a1".into(),
                name: "failed-rows.csv".into(),
                kind: AttachmentKind::File,
                detail: Some("CSV, 37 lines".into()),
                uploading: false,
            },
            ComposerAttachment {
                id: "a2".into(),
                name: "supplier-feed.json".into(),
                kind: AttachmentKind::File,
                detail: None,
                uploading: true,
            },
        ];
        let mut stacked = kept(ui, "stacked", "What do these rows have in common?");
        let _ = Composer::new("story-stacked", &mut stacked)
            .attach(true)
            .attachments(&files)
            .show(ui, |_| {});
        keep(ui, "stacked", stacked);
        ui.add_space(20.0);
        section(ui, "Stacked and empty, shut until two models are chosen");
        let mut empty = String::new();
        let _ = Composer::new("story-disabled", &mut empty)
            .disabled(true)
            .words(ComposerWords {
                placeholder: "Choose two models first".into(),
                send: "Run".into(),
                ..ComposerWords::default()
            })
            .show(ui, |_| {});
    });
}

fn conversations() -> Vec<ConversationEntry> {
    vec![
        ConversationEntry {
            id: "c-cron".into(),
            title: "Weekday cron for send-digest".into(),
            time: NOW - 3 * 60 * MINUTE,
            pinned: true,
        },
        ConversationEntry {
            id: "c-parse".into(),
            title: "Amounts with a decimal comma".into(),
            time: NOW - 25 * MINUTE,
            pinned: false,
        },
        ConversationEntry {
            id: "c-week".into(),
            title: "This week's runs".into(),
            time: NOW - DAY,
            pinned: false,
        },
        ConversationEntry {
            id: "c-slow".into(),
            title: "Why a bigger model writes slower".into(),
            time: NOW - 6 * DAY,
            pinned: false,
        },
        ConversationEntry {
            id: "c-old".into(),
            title: "Harbor's export, last year".into(),
            time: NOW - 400 * DAY,
            pinned: false,
        },
    ]
}

pub fn conversation_history(_: &mut State, ui: &mut Ui) {
    section(
        ui,
        "Pinned, today's and earlier, the current one marked; unframed beside a chat",
    );
    let all = conversations();
    ui.allocate_ui(vec2(308.0, 420.0), |ui| {
        ui.set_width(308.0);
        let _ = ConversationHistory::new("story-history", &all, NOW)
            .current(Some("c-parse"))
            .framed(false)
            .show(ui);
    });
}

pub fn chat(_: &mut State, ui: &mut Ui) {
    let all = conversations();
    let height = ui.available_height().min(820.0);
    let width = ui.available_width();
    ui.allocate_ui(vec2(width, height), |ui| {
        ui.set_height(height);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.allocate_ui(vec2(280.0, height), |ui| {
                ui.set_width(280.0);
                let _ = ConversationHistory::new("story-chat-history", &all, NOW)
                    .current(Some("c-parse"))
                    .framed(false)
                    .show(ui);
            });
            ui.allocate_ui(vec2(width - 280.0, height), |ui| {
                ui.set_width(width - 280.0);
                ui.set_height(height);
                let mut draft = kept(ui, "chat", "");
                Conversation::new("story-conversation").show(
                    ui,
                    |ui| {
                        PaneHeader::new()
                            .title_with(|ui| {
                                ui.add(IconButton::new(Icon::History, "Hide conversations").pressed(true));
                                ui.label(
                                    sokkel_egui::TextRole::SectionTitle
                                        .rich(&sokkel_egui::tokens(ui.ctx()), "Amounts with a decimal comma"),
                                );
                            })
                            .show(ui, |ui| {
                                ui.add(IconButton::new(Icon::MessageSquarePlus, "New chat"));
                            });
                    },
                    |ui| {
                        ui.spacing_mut().item_spacing.y = 10.0;
                        ui.add(
                            Bubble::new("Now a jq query for the failed runs in the API answer.")
                                .variant(BubbleVariant::Secondary)
                                .end(true),
                        );
                        let _ = ReactionChips::new("story-chat-reactions", &[]).end(true).show(ui);
                        ui.add_space(14.0);
                        let _ = AssistantReply::new("story-chat-reply", JQ_ANSWER)
                            .mark(Icon::Bot)
                            .name("brine-coder-14b")
                            .code_name(true)
                            .time("08:32")
                            .retry(true)
                            .feedback(None)
                            .show(ui, |ui| {
                                ui.add(
                                    UsageLine::new(72.0)
                                        .input(257.0)
                                        .first_token(230.0)
                                        .rate(30.4)
                                        .latency(2600.0),
                                );
                            });
                    },
                    |ui| {
                        let _ = Composer::new("story-chat-composer", &mut draft)
                            .layout(ComposerLayout::Inline)
                            .attach(true)
                            .words(ComposerWords {
                                placeholder: "Ask brine-coder-14b".into(),
                                ..ComposerWords::default()
                            })
                            .show(ui, model_tool);
                    },
                );
                keep(ui, "chat", draft);
            });
        });
    });
}
