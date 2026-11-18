//! Markdown drawn as the notes example draws a note: "Start here" at the document scale with its
//! links and tags, every block it reads in light and dark, a backlink's line marked, and presses on
//! a link, a tag and an address coming back in the response, and a heading revealed.

use egui::{Frame, Margin, ScrollArea};
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_egui::{Appearance, Markdown, Scheme, TextRole, install};

/// The `welcome` note's body, as the notes example seeds it.
const WELCOME: &str = r#"# Start here

This vault holds what the team knows about Tidevann's jobs: how they run, what to do when one
fails, and what was said about them.

- The [[Runbooks]] say what to do when something breaks: start with [[Retry a run]].
- Each job has a note: [[nightly-export]], [[sync-inventory]], [[rebuild-index]], [[send-digest]]
  and [[import-orders]].
- The projects are [[harbor]], [[juniper]] and [[larkspur]].
- The daily notes are the journal; open today's from **Daily** in the rail.

Type `[[` in the editor to link to another note, and `#` to tag one. #welcome
"#;

/// Every block the widget reads, at the message scale.
const PARTS: &str = r#"## Retry a run

A failed run starts again from its failed step once the cause is fixed; see [[Scheduler is stuck]]
for runs that stay queued, or [the run docs](https://example.org/docs/runs).

1. Open the run and read the failed step's log.
2. Press **Run again from the failed step**.
   - the steps before it keep their output
   - *validate* fails the same way if the fix did not hold

- [x] Read the failed nightly-export from last night
- [ ] Ask linus about the ~~upload~~ validate step

> [!WARNING]
> Never retry nightly-export twice at once: the second run overwrites the first one's upload.

> The scheduler picks runs up again within a minute.

```sh
tidevann runs retry nightly-export --from validate
```

---

### Who answers

margaret, then grace. #on-call
"#;

/// Whether the vault holds a note: every name but one the parts link to.
fn exists(target: &str) -> bool {
    target != "Scheduler is stuck"
}

#[derive(Default)]
struct Pressed {
    opened: Option<(String, Option<String>)>,
    tag: Option<String>,
    url: Option<String>,
}

fn note() -> Harness<'static, Pressed> {
    let mut harness = Harness::builder().with_size([429.0, 400.0]).build_ui_state(
        |ui, pressed: &mut Pressed| {
            install(ui.ctx(), &Appearance::default());
            Frame::NONE.inner_margin(Margin::same(12)).show(ui, |ui| {
                let shown = Markdown::new(WELCOME)
                    .scale("document".parse().unwrap())
                    .anchors(true)
                    .links(&|_| true)
                    .tags(true)
                    .open_urls(false)
                    .show(ui);
                pressed.opened = shown.opened.or(pressed.opened.take());
                pressed.tag = shown.tag.or(pressed.tag.take());
                pressed.url = shown.url.or(pressed.url.take());
            });
        },
        Pressed::default(),
    );
    harness.run_steps(3);
    harness
}

#[test]
fn start_here_as_a_document() {
    let mut harness = note();
    harness.snapshot("markdown_note");
}

#[test]
fn a_press_on_a_link_opens_its_note() {
    let mut harness = note();
    harness.get_by_label("Retry a run").click();
    harness.run_steps(2);
    assert_eq!(harness.state().opened, Some(("Retry a run".into(), None)));
}

#[test]
fn a_press_on_a_tag_follows_it() {
    let mut harness = note();
    harness.get_by_label("#welcome").click();
    harness.run_steps(2);
    assert_eq!(harness.state().tag.as_deref(), Some("welcome"));
}

fn parts(scheme: Scheme) -> Harness<'static, Pressed> {
    let mut harness = Harness::builder().with_size([480.0, 720.0]).build_ui_state(
        move |ui, pressed: &mut Pressed| {
            install(
                ui.ctx(),
                &Appearance {
                    scheme,
                    ..Default::default()
                },
            );
            Frame::NONE.inner_margin(Margin::same(12)).show(ui, |ui| {
                let shown = Markdown::new(PARTS).links(&exists).tags(true).open_urls(false).show(ui);
                pressed.url = shown.url.or(pressed.url.take());
            });
        },
        Pressed::default(),
    );
    harness.run_steps(3);
    harness
}

#[test]
fn every_block_in_light() {
    let mut harness = parts(Scheme::Light);
    harness.snapshot("markdown_parts_light");
}

#[test]
fn every_block_in_dark() {
    let mut harness = parts(Scheme::Dark);
    harness.snapshot("markdown_parts_dark");
}

#[test]
fn a_press_on_an_address_comes_back() {
    let mut harness = parts(Scheme::Light);
    harness.get_by_label("the run docs").click();
    harness.run_steps(2);
    assert_eq!(harness.state().url.as_deref(), Some("https://example.org/docs/runs"));
}

#[test]
fn a_backlink_line_marked() {
    let mut harness = Harness::builder().with_size([300.0, 64.0]).build_ui(|ui| {
        install(ui.ctx(), &Appearance::default());
        Frame::NONE.inner_margin(Margin::same(8)).show(ui, |ui| {
            Markdown::new("First day keeping a journal in the vault. Started [[welcome|Start here]].")
                .inline(TextRole::Help, "muted-foreground")
                .live(false)
                .highlight("start here")
                .show(ui);
        });
    });
    harness.run_steps(3);
    assert!(
        harness.query_by_label("Start here").is_none(),
        "a line that is not live holds no link"
    );
    harness.snapshot("markdown_backlink");
}

#[test]
fn a_heading_is_revealed() {
    let long = format!(
        "{}\n\n## Who answers\n\nmargaret, then grace.",
        "A line of the note.\n\n".repeat(30)
    );
    let mut harness = Harness::builder().with_size([360.0, 200.0]).build_ui_state(
        move |ui, top: &mut Option<f32>| {
            install(ui.ctx(), &Appearance::default());
            ScrollArea::vertical().show(ui, |ui| {
                let shown = Markdown::new(&long)
                    .scale("document".parse().unwrap())
                    .anchors(true)
                    .reveal(Some("who-answers"))
                    .show(ui);
                *top = shown.headings.first().map(|(_, rect)| rect.top());
            });
        },
        None,
    );
    harness.run_steps(4);
    let top = harness.state().expect("the heading is listed");
    assert!((0.0..200.0).contains(&top), "the heading is in view, at {top}");
}
