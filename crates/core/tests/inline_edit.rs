//! A page title edited in place: pressed, typed, saved with Enter; Escape drops what was typed;
//! a title refused stays open with its message until it changes.

use egui::Key;
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_egui::{Appearance, PageHeader, install};

struct Page {
    title: String,
}

fn page() -> Harness<'static, Page> {
    let mut harness = Harness::builder().with_size([640.0, 200.0]).build_ui_state(
        |ui, page: &mut Page| {
            install(ui.ctx(), &Appearance::default());
            let head = PageHeader::new(&page.title)
                .edit_title("Title", |title| {
                    title.trim().is_empty().then(|| "An incident needs a title.".to_owned())
                })
                .show(ui, |_| {});
            if let Some(title) = head.renamed {
                page.title = title;
            }
        },
        Page {
            title: "Slow exports from storage".into(),
        },
    );
    harness.run_steps(2);
    harness
}

fn open(harness: &mut Harness<'static, Page>) {
    harness.get_by_label("Title Edit").click();
    harness.run_steps(2);
}

#[test]
fn enter_saves_what_was_typed() {
    let mut harness = page();
    open(&mut harness);
    harness.get_by_label("Title").type_text("Exports slow to start");
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(harness.state().title, "Exports slow to start");
    assert!(
        harness.query_by_label("Title Edit").is_some(),
        "and the title is shown again"
    );
}

#[test]
fn escape_drops_what_was_typed() {
    let mut harness = page();
    open(&mut harness);
    harness.get_by_label("Title").type_text("Something else");
    harness.key_press(Key::Escape);
    harness.run_steps(2);
    assert_eq!(harness.state().title, "Slow exports from storage");
    assert!(harness.query_by_label("Title Edit").is_some());
}

#[test]
fn an_empty_title_is_refused_with_its_message() {
    let mut harness = page();
    open(&mut harness);
    harness.key_press(Key::Backspace);
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(harness.state().title, "Slow exports from storage");
    assert!(harness.query_all_by_label("An incident needs a title.").count() > 0);
    assert!(harness.query_by_label("Title Edit").is_none(), "the editor stays open");
}
