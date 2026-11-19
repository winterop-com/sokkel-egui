//! What is being edited, as the web's `editing.ts` keeps it: each resource's unsaved text and the
//! version it was read at (kept in the browser, so an edit survives a reload), and the server's
//! last word on each. The resource's page and its side panel read the same.

use serde::Serialize;

use crate::model::{Issue, Part};

/// An unsaved edit and the version it started from, sent as If-Match.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Draft {
    pub text: String,
    #[serde(rename = "readAt")]
    pub read_at: f64,
}

/// Where the editor is asked to go: a line and its columns, or the value at a path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Reveal {
    Line { line: usize, from: usize, to: usize },
    Path { path: Vec<Part> },
}

/// The edits and the server's words, each kept under its key in the order first kept, as a
/// JavaScript object keeps its keys.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Editing {
    pub drafts: Vec<(String, Draft)>,
    pub answers: Vec<(String, Vec<Issue>)>,
    pub reveal: Option<Reveal>,
}

impl Editing {
    pub fn draft(&self, key: &str) -> Option<&Draft> {
        self.drafts.iter().find(|(one, _)| one == key).map(|(_, draft)| draft)
    }

    pub fn set_draft(&mut self, key: &str, draft: Option<Draft>) {
        let at = self.drafts.iter().position(|(one, _)| one == key);
        match (draft, at) {
            (None, Some(at)) => {
                self.drafts.remove(at);
            }
            (None, None) => {}
            (Some(draft), Some(at)) => self.drafts[at].1 = draft,
            (Some(draft), None) => self.drafts.push((key.to_owned(), draft)),
        }
    }

    pub fn answer(&self, key: &str) -> &[Issue] {
        self.answers
            .iter()
            .find(|(one, _)| one == key)
            .map_or(&[], |(_, issues)| issues.as_slice())
    }

    pub fn set_answer(&mut self, key: &str, issues: Vec<Issue>) {
        match self.answers.iter().position(|(one, _)| one == key) {
            Some(at) => self.answers[at].1 = issues,
            None => self.answers.push((key.to_owned(), issues)),
        }
    }
}
