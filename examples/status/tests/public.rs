//! The public page's "Get notified": the address checked as the web's form checks it, every
//! service or some, and the address said back once subscribed.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_egui::Appearance;
use status::StatusApp;

fn render(height: f32) -> Harness<'static, StatusApp> {
    let mut app = StatusApp::on(Appearance::default());
    app.tick();
    app.tick();
    app.go("public");
    let mut harness = Harness::builder().with_size([1280.0, height]).build_ui_state(
        |ui, app: &mut StatusApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(4);
    harness
}

#[test]
fn the_whole_page() {
    let mut harness = render(2600.0);
    harness.snapshot("public_whole");
}

#[test]
fn an_address_is_checked_then_subscribed() {
    let mut app = StatusApp::on(Appearance::default());
    let form = &mut app.subscribe;
    form.email = "ada".into();
    form.submit();
    assert_eq!(form.error.as_deref(), Some("Write an address such as you@example.org."));
    form.email = " Ada@Example.org ".into();
    form.which = "some".into();
    form.submit();
    assert_eq!(
        form.error.as_deref(),
        Some("Tick at least one service, or choose every service.")
    );
    form.picked[0] = true;
    form.error = None;
    form.submit();
    assert_eq!(form.error, None);
    let done = form.done.clone().expect("subscribed");
    assert_eq!(done.email, "ada@example.org");
    assert_eq!(done.services, vec!["web"]);
    form.done = None;
    form.email = "ada@example.org".into();
    form.submit();
    assert_eq!(form.error.as_deref(), Some("ada@example.org is subscribed already."));
}

#[test]
fn some_services_shows_the_boxes() {
    let mut harness = render(2600.0);
    // the uptime list names each service once; the boxes name it again
    let before = harness.query_all_by_label("scheduler").count();
    harness.get_by_label("Some services").click();
    harness.run_steps(3);
    assert_eq!(harness.state().subscribe.which, "some");
    assert_eq!(harness.query_all_by_label("scheduler").count(), before + 1);
}

#[test]
fn subscribe_scrolls_to_the_form() {
    let mut harness = render(800.0);
    // the bar's Subscribe comes first; the form's is the second
    harness
        .get_all_by_label("Subscribe")
        .next()
        .expect("the bar's Subscribe")
        .click();
    harness.run_steps(4);
    assert!(!harness.state().subscribe.reveal);
    // the form's address field is now on the screen
    let field = harness.get_by_label("Email");
    assert!(field.rect().min.y < 800.0 && field.rect().min.y > 0.0);
}
