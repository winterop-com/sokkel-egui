//! What the pages share, as the web's `ui.tsx`: a form's kind chip, a state's color, an answer in
//! words, and dates and counts as the web writes them.

use eframe::egui::Color32;
use sokkel_egui::{Chip, ChipTone, Tokens};

use crate::backend::ReceiptState;
use crate::forms::{AnswerValue, Form, Question};

/// A form's kind as a chip in its kind's color.
pub fn kind_chip(form: &Form) -> Chip {
    Chip::new(form.kind.label()).tone(ChipTone::Kind(form.kind.color()))
}

/// The status token a state is drawn in.
pub fn state_status(state: ReceiptState) -> &'static str {
    match state {
        ReceiptState::Received => "running",
        ReceiptState::Forwarded => "succeeded",
        ReceiptState::Rejected => "failed",
        ReceiptState::Withdrawn => "cancelled",
    }
}

/// A state's color, for a dot.
pub fn state_dot(t: &Tokens, state: ReceiptState) -> Color32 {
    t.color(&format!("status-{}", state_status(state)))
}

/// A state's chip tone, in the app's colors.
pub fn state_tone(state: ReceiptState) -> ChipTone {
    match state {
        ReceiptState::Received => ChipTone::Info,
        ReceiptState::Forwarded => ChipTone::Good,
        ReceiptState::Rejected => ChipTone::Critical,
        ReceiptState::Withdrawn => ChipTone::Neutral,
    }
}

/// A state as its chip: the state's words in the app's colors.
pub fn state_chip(state: ReceiptState) -> Chip {
    Chip::new(state.key()).tone(state_tone(state))
}

/// A number with its thousands grouped, as `Intl.NumberFormat('en-US')` writes it.
pub fn grouped(value: f64) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    let whole = rounded.trunc().abs() as u64;
    let mut digits = whole.to_string();
    let mut out = String::new();
    while digits.len() > 3 {
        let tail = digits.split_off(digits.len() - 3);
        out = format!(",{tail}{out}");
    }
    out = format!("{digits}{out}");
    let fraction = rounded.abs().fract();
    if fraction > 0.0 {
        let text = format!("{fraction:.3}");
        out.push_str(text.trim_start_matches('0').trim_end_matches('0'));
    }
    if rounded < 0.0 { format!("-{out}") } else { out }
}

/// The plural of a count's noun.
pub fn count(value: usize, one: &str) -> String {
    count_as(value, one, &format!("{one}s"))
}

pub fn count_as(value: usize, one: &str, many: &str) -> String {
    format!("{} {}", grouped(value as f64), if value == 1 { one } else { many })
}

/// An answer as the record shows it: yes or no in words, a choice by its label, a number with
/// its unit.
pub fn answer_text(question: Option<&Question>, value: Option<&AnswerValue>) -> String {
    match value {
        None => String::new(),
        Some(AnswerValue::Text(t)) if t.is_empty() => String::new(),
        Some(AnswerValue::List(l)) => l
            .iter()
            .filter(|one| !one.trim().is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(", "),
        Some(AnswerValue::Bool(b)) => if *b { "Yes" } else { "No" }.into(),
        Some(value) if question.is_some_and(|q| !q.options.is_empty()) => {
            let written = value.written();
            question
                .and_then(|q| q.options.iter().find(|o| o.value == written))
                .map_or(written, |o| o.label.to_owned())
        }
        Some(AnswerValue::Number(n)) => {
            let unit = question.and_then(|q| q.unit).filter(|u| !u.contains(" to "));
            format!("{}{}", grouped(*n), unit.map(|u| format!(" {u}")).unwrap_or_default())
        }
        Some(value) => value.written(),
    }
}

/// A moment as a date and a time in the local zone: `Nov 18, 2026, 10:13 AM`.
pub fn when(at: i64) -> String {
    use crate::days::{DAY, day_of, local_offset};
    let minutes = (at + local_offset()).rem_euclid(DAY) / 60_000;
    let (hour, minute) = (minutes / 60, minutes % 60);
    let half = if hour < 12 { "AM" } else { "PM" };
    let shown = if hour % 12 == 0 { 12 } else { hour % 12 };
    format!("{}, {shown}:{minute:02} {half}", crate::days::long_day(day_of(at)))
}

/// A `YYYY-MM-DD` day as a date: `Nov 18, 2026`.
pub fn on_day(iso: &str) -> String {
    crate::days::day_from_iso(&iso[..iso.len().min(10)]).map_or_else(|| iso.to_owned(), crate::days::long_day)
}
