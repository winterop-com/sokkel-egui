//! The chat page's own logic, as the web's `pages/Chat.tsx` and sokkel-ui's `ConversationHistory`
//! have it: where a conversation is listed and the time it shows, what an answer shows while it
//! streams and once kept, and the suggestions a new conversation offers.

use serde::Serialize;

use crate::data::model::{AssistantMessage, Conversation, Model, ReplyState, Usage, Version};
use crate::engine::chat::chat_key;
use crate::engine::stream::{Live, Phase, usage_of};
use crate::js::{MONTHS, date_medium, start_of_day, utc_parts};

/// What a new conversation offers to ask: a title and its prompt.
pub const SUGGESTIONS: [(&str, &str); 4] = [
    (
        "Write a parser",
        "Write a Python function to parse amounts that may use a decimal comma, like 12,50.",
    ),
    (
        "Explain a slowdown",
        "Explain why a bigger model writes slower than a small one.",
    ),
    (
        "A cron schedule",
        "What cron schedule runs send-digest every weekday at 06:30?",
    ),
    (
        "Summarize a week",
        "Summarize this week for the harbor project: failures, timeouts and anything slow.",
    ),
];

/// Where a conversation is listed: pinned ones first, then today's, then the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Group {
    Pinned,
    Today,
    Earlier,
}

impl Group {
    pub fn label(self) -> &'static str {
        match self {
            Group::Pinned => "Pinned",
            Group::Today => "Today",
            Group::Earlier => "Earlier",
        }
    }
}

/// Where a conversation is listed, read against `now`.
pub fn conversation_group(pinned: bool, time: i64, now: i64) -> Group {
    if pinned {
        Group::Pinned
    } else if time >= start_of_day(now) {
        Group::Today
    } else {
        Group::Earlier
    }
}

/// The time a conversation shows: the hour today, the day this year, the date before.
pub fn conversation_time(time: i64, now: i64) -> String {
    let (year, month, day, hour, minute, _) = utc_parts(time);
    if time >= start_of_day(now) {
        let half = if hour < 12 { "AM" } else { "PM" };
        let twelve = if hour % 12 == 0 { 12 } else { hour % 12 };
        return format!("{twelve}:{minute:02} {half}");
    }
    if year == utc_parts(now).0 {
        format!("{} {day}", MONTHS[(month - 1) as usize])
    } else {
        date_medium(time)
    }
}

/// One conversation in the history: its group, its time as shown, and its menu's words.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HistoryRow {
    pub group: Group,
    pub id: String,
    pub time: String,
    pub actions: [&'static str; 3],
}

/// The conversations newest first, each with its group, its time and its menu, as the history
/// lists them; the table draws them under the groups in Pinned, Today, Earlier order.
pub fn history(conversations: &[Conversation], now: i64) -> Vec<HistoryRow> {
    let mut rows: Vec<&Conversation> = conversations.iter().collect();
    rows.sort_by_key(|one| std::cmp::Reverse(one.time));
    rows.into_iter()
        .map(|one| HistoryRow {
            group: conversation_group(one.pinned, one.time, now),
            id: one.id.clone(),
            time: conversation_time(one.time, now),
            actions: [if one.pinned { "Unpin" } else { "Pin" }, "Rename", "Delete"],
        })
        .collect()
}

/// What an answer shows: from what is being written while it streams, else from what was kept.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Reply {
    pub name: String,
    pub text: String,
    pub reasoning: Option<String>,
    pub streaming: bool,
    pub state: ReplyState,
    pub usage: Option<Usage>,
    pub problem: Option<String>,
    /// What it says while it thinks: loading the model, reading the prompt, or thinking.
    pub thinking: String,
    /// The version shown from 1, and how many there are, when there is more than one.
    pub versions: Option<(usize, usize)>,
}

pub fn reply(
    conversation: &Conversation,
    message: &AssistantMessage,
    models: &[Model],
    live: &[(String, Live)],
    now: i64,
) -> Reply {
    let version: &Version = &message.versions[message.at];
    let key = chat_key(&conversation.id, &message.id, message.at);
    let writing = live.iter().find(|(one, _)| *one == key).map(|(_, live)| live);
    let model_id = writing.map_or(&version.model, |live| &live.model);
    let name = models
        .iter()
        .find(|one| one.id == *model_id)
        .map_or_else(|| version.model.clone(), |model| model.name.clone());
    let text = writing.map_or_else(|| version.text.clone(), |live| live.text.clone());
    let reasoning = writing.map_or_else(|| version.reasoning.clone(), |live| Some(live.reasoning.clone()));
    let streaming = match writing {
        Some(live) => !live.phase.over(),
        None => version.state == ReplyState::Streaming,
    };
    let state = if streaming {
        ReplyState::Streaming
    } else {
        match writing.map(|live| live.phase) {
            Some(Phase::Failed) => ReplyState::Failed,
            Some(Phase::Stopped) => ReplyState::Stopped,
            _ => version.state,
        }
    };
    let usage = match writing {
        Some(live) if live.first.is_some() => Some(usage_of(live, now)),
        _ => version.usage.clone(),
    };
    let problem = writing
        .and_then(|live| live.problem.clone())
        .or_else(|| version.problem.clone());
    let thinking = match writing.map(|live| live.phase) {
        Some(Phase::Loading) => format!("Loading {name} into memory"),
        Some(Phase::Waiting) => "Reading the prompt".to_owned(),
        _ => "Thinking".to_owned(),
    };
    Reply {
        reasoning: reasoning.filter(|said| !said.is_empty()),
        name,
        text,
        streaming,
        state,
        usage,
        problem,
        thinking,
        versions: (message.versions.len() > 1).then(|| (message.at + 1, message.versions.len())),
    }
}

/// The composer's placeholder for a conversation's model.
pub fn placeholder(model: Option<&Model>) -> String {
    model.map_or_else(
        || "Pick a model to ask".to_owned(),
        |model| format!("Ask {}", model.name),
    )
}
