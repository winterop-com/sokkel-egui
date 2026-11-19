//! The app's screens at the web's routes, at 1440 by 900 on the parity day (`parity/models.json`),
//! each a second after the page opened as the backend's first answers come: `bun scripts/parity.ts
//! board models` puts them beside the web's. Then a few things the reader does: a message sent in
//! a chat is answered token by token as the clock moves, a pull runs and lands in the library, a
//! comparison runs three models, and stopping the runner unloads what it held.

use egui_kittest::{Harness, SnapshotResults};
use models::ModelsApp;
use models::data::catalog::entry_of;
use models::data::model::{DownloadState, LoadState, Message, ReplyState, RunnerStatus};
use models::engine::chat::chat_key;
use models::workbench::DEMO_START;
use sokkel_egui::Appearance;

pub fn render(path: &str) -> Harness<'static, ModelsApp> {
    let mut app = ModelsApp::at(DEMO_START, Appearance::default());
    app.go(path);
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut ModelsApp| {
            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    settle(&mut harness);
    harness
}

/// Moves the clock on past the backend's answers and draws again.
fn settle(harness: &mut Harness<'static, ModelsApp>) {
    harness.run_steps(2);
    harness.state_mut().wait(1_000);
    harness.run_steps(4);
}

/// Moves the clock on by `ms`, a step at a time as frames come, and draws.
fn wait(harness: &mut Harness<'static, ModelsApp>, ms: i64) {
    let mut left = ms;
    while left > 0 {
        let step = left.min(250);
        harness.state_mut().wait(step);
        harness.run_steps(1);
        left -= step;
    }
    harness.run_steps(2);
}

#[test]
fn every_screen() {
    let mut results = SnapshotResults::new();
    for (name, path) in [
        ("models_library", "library"),
        ("models_model", "library/brine-coder-14b-4bit"),
        ("models_disk", "disk"),
        ("models_discover", "discover"),
        ("models_downloads", "downloads"),
        ("models_chat", "chat/c-parse"),
        ("models_compare", "compare/cmp-seed"),
        ("models_runner", "runner"),
    ] {
        let mut harness = render(path);
        results.add(harness.try_snapshot(name));
    }
    let mut harness = render("library");
    harness.state_mut().go("settings");
    harness.run_steps(4);
    results.add(harness.try_snapshot("models_settings"));
}

#[test]
fn a_message_sent_is_answered_token_by_token() {
    let mut harness = render("chat/c-cron");
    let before = harness
        .state()
        .backend
        .conversations
        .seen
        .iter()
        .find(|one| one.id == "c-cron")
        .map_or(0, |one| one.messages.len());
    harness
        .state_mut()
        .backend
        .send("c-cron", "Now the same at 07:15 on Saturdays.", Vec::new());
    harness.run_steps(2);
    // a short while in, the answer is being written: some words, not all
    wait(&mut harness, 1_500);
    let app = harness.state();
    let talk = app
        .backend
        .conversations
        .seen
        .iter()
        .find(|one| one.id == "c-cron")
        .expect("c-cron");
    assert_eq!(talk.messages.len(), before + 2, "the question and its answer");
    let Some(Message::Assistant(answer)) = talk.messages.last() else {
        panic!("an answer last")
    };
    let key = chat_key("c-cron", &answer.id, answer.at);
    let early = app
        .backend
        .live
        .iter()
        .find(|(one, _)| *one == key)
        .map(|(_, live)| live.text.clone())
        .unwrap_or_default();
    // later it has more, and once done the conversation keeps it
    wait(&mut harness, 30_000);
    let app = harness.state();
    let talk = app
        .backend
        .conversations
        .seen
        .iter()
        .find(|one| one.id == "c-cron")
        .expect("c-cron");
    let Some(Message::Assistant(answer)) = talk.messages.last() else {
        panic!("an answer last")
    };
    let kept = &answer.versions[answer.at];
    assert_eq!(kept.state, ReplyState::Done);
    assert!(kept.text.len() > early.len(), "{early:?} grew into {:?}", kept.text);
    assert!(kept.text.starts_with(early.trim_end()), "it streamed the same words");
    assert!(kept.usage.is_some());
}

#[test]
fn a_pull_runs_and_lands_in_the_library() {
    let mut harness = render("downloads");
    let entry = entry_of("ada/tide-32b-instruct").expect("tide-32b-instruct in the catalog");
    let at = entry.variants.iter().position(|one| one.quant == "Q4_K_M").unwrap_or(0);
    harness.state_mut().backend.pull(entry, &entry.variants[at]);
    wait(&mut harness, 2_000);
    let pulling = harness
        .state()
        .backend
        .downloads
        .seen
        .iter()
        .find(|one| one.name == "tide-32b-instruct")
        .cloned()
        .expect("a pull");
    assert_eq!(pulling.state, DownloadState::Running);
    assert!(pulling.done > 0.0 && pulling.done < pulling.size);
    // the rail counts it while it runs
    let (groups, _) = harness.state().rail();
    assert!(
        groups
            .iter()
            .flat_map(|group| group.entries.iter())
            .any(|entry| entry.id == "downloads" && entry.count == Some(1))
    );
    wait(&mut harness, 600_000);
    let app = harness.state();
    let done = app
        .backend
        .downloads
        .seen
        .iter()
        .find(|one| one.name == "tide-32b-instruct")
        .expect("the pull");
    assert_eq!(done.state, DownloadState::Done);
    assert!(
        app.backend
            .models
            .seen
            .iter()
            .any(|one| one.id == "tide-32b-instruct-q4_k_m")
    );
}

#[test]
fn a_comparison_runs_three_models() {
    let mut harness = render("compare/new");
    let draft = harness.state().kept.compare.clone();
    let promise = harness.state_mut().backend.compare(
        "Explain why sync-inventory ran slow on Tuesday.",
        &draft.models,
        draft.temperature,
        draft.max_tokens,
    );
    let _ = promise;
    wait(&mut harness, 120_000);
    let app = harness.state();
    let made = app
        .backend
        .comparisons
        .seen
        .iter()
        .find(|one| one.prompt.contains("sync-inventory"))
        .expect("the comparison");
    assert_eq!(made.columns.len(), 3);
    assert!(
        made.columns.iter().all(|one| !one.text.is_empty()),
        "every model answered"
    );
}

#[test]
fn stopping_the_runner_unloads_its_models() {
    let mut harness = render("runner");
    assert!(
        harness
            .state()
            .backend
            .loaded
            .seen
            .iter()
            .any(|one| one.state == LoadState::Ready)
    );
    harness.state_mut().backend.stop_runner();
    wait(&mut harness, 10_000);
    let app = harness.state();
    assert_eq!(app.backend.runner.seen[0].status, RunnerStatus::Down);
    assert!(app.backend.loaded.seen.is_empty());
}
