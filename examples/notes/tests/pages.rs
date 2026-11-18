//! The screens parity/notes.json lists, drawn as the web's are shot: the vault opened at
//! 2026-11-18 09:00 UTC. `bun scripts/parity.ts board notes` puts them beside the web's.

use egui_kittest::{Harness, SnapshotResults};
use notes::NotesApp;
use notes::app::DEMO_START;
use sokkel_egui::Appearance;

pub fn render(path: &str) -> Harness<'static, NotesApp> {
    let mut app = NotesApp::at(DEMO_START, Appearance::default());
    app.go(path);
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut NotesApp| {
            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(4);
    harness
}

#[test]
fn every_screen() {
    let mut results = SnapshotResults::new();
    for (name, path) in [
        ("notes_all", "notes"),
        ("notes_folder", "folder/runbooks"),
        ("notes_note", "note/welcome"),
        ("notes_daily", "daily"),
        ("notes_tags", "tags"),
        ("notes_graph", "graph"),
        ("notes_search", "search"),
        ("notes_search_found", "search/export"),
        ("notes_settings", "settings"),
        ("notes_settings_daily", "settings/daily"),
    ] {
        let mut harness = render(path);
        results.add(harness.try_snapshot(name));
    }
}

#[test]
fn every_dialog() {
    use notes::backend::Asking;
    let mut results = SnapshotResults::new();
    let asks = [
        (
            "notes_dialog_rename",
            "note/welcome",
            Asking::RenameNote { id: "welcome".into() },
        ),
        (
            "notes_dialog_move",
            "note/welcome",
            Asking::MoveNote { id: "welcome".into() },
        ),
        (
            "notes_dialog_delete_folder",
            "folder/runbooks",
            Asking::DeleteFolder { id: "runbooks".into() },
        ),
        (
            "notes_dialog_make",
            "note/welcome",
            Asking::Create {
                title: "Export retention".into(),
                folder: None,
            },
        ),
    ];
    for (name, path, asking) in asks {
        let mut harness = render(path);
        harness.state_mut().ask(asking);
        harness.run_steps(4);
        results.add(harness.try_snapshot(name));
    }
}

#[test]
fn the_palettes() {
    let mut results = SnapshotResults::new();
    let mut harness = render("notes");
    harness.state_mut().palette = Some(Default::default());
    harness.run_steps(4);
    results.add(harness.try_snapshot("notes_palette"));
    let mut harness = render("note/welcome");
    harness.state_mut().switching = Some(("export ret".into(), 0));
    harness.run_steps(4);
    results.add(harness.try_snapshot("notes_switcher_make"));
}

#[test]
fn the_top_search() {
    use egui_kittest::kittest::Queryable;
    let mut results = SnapshotResults::new();
    let mut harness = render("notes");
    // the top bar's field, after the rail's filter: typed in, its results open under it
    let fields: Vec<_> = harness
        .query_all_by_role(eframe::egui::accesskit::Role::TextInput)
        .collect();
    fields[1].click();
    harness.run_steps(1);
    harness
        .get_all_by_role(eframe::egui::accesskit::Role::TextInput)
        .nth(1)
        .unwrap()
        .type_text("export");
    harness.run_steps(4);
    results.add(harness.try_snapshot("notes_top_search"));
}

#[test]
fn a_name_already_taken() {
    use egui_kittest::kittest::Queryable;
    let mut results = SnapshotResults::new();
    let mut harness = render("notes");
    let input = || eframe::egui::accesskit::Role::TextInput;
    let field = harness.query_all_by_role_and_label(input(), "New note").next().unwrap();
    field.click();
    harness.run_steps(1);
    harness
        .query_all_by_role_and_label(input(), "New note")
        .next()
        .unwrap()
        .type_text("Ideas");
    harness.run_steps(1);
    harness.key_press(eframe::egui::Key::Enter);
    harness.run_steps(3);
    results.add(harness.try_snapshot("notes_quick_add_taken"));
}

#[test]
fn a_note_dragged_into_a_folder() {
    use eframe::egui::{Event, Modifiers, PointerButton, pos2};
    use egui_kittest::kittest::Queryable;
    let mut harness = render("notes");
    let row = |h: &Harness<'static, NotesApp>, label: &str| {
        h.query_all_by_label(label)
            .map(|n| n.rect())
            .find(|r| r.left() < 240.0)
            .unwrap()
    };
    let from = row(&harness, "Ideas").center();
    let onto = row(&harness, "Runbooks");
    let to = pos2(onto.center().x, onto.top() + onto.height() * 0.75);
    let press = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.input_mut().events.push(Event::PointerMoved(from));
    harness.input_mut().events.push(press(from, true));
    harness.run_steps(1);
    for step in 1..=10 {
        let at = from + (to - from) * (step as f32 / 10.0);
        harness.input_mut().events.push(Event::PointerMoved(at));
        harness.run_steps(1);
    }
    harness.input_mut().events.push(press(to, false));
    harness.run_steps(3);
    let vault = harness.state().backend.vault();
    let ideas = vault.notes.iter().find(|n| n.title == "Ideas").unwrap();
    assert_eq!(ideas.folder.as_deref(), Some("runbooks"));
}
