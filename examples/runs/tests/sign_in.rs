//! Signing out shows the door; ada with the password "tide" opens the console again, and a wrong
//! password is said.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use runs::RunsApp;
use runs::app::DEMO_START;
use sokkel_egui::Appearance;

fn render() -> Harness<'static, RunsApp> {
    let mut app = RunsApp::at(DEMO_START, Appearance::default());
    app.sign_out();
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut RunsApp| {
            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(4);
    harness
}

fn sign_in(harness: &mut Harness<'static, RunsApp>, user: &str, password: &str) {
    use eframe::egui::accesskit::Role;
    harness.get_by_role_and_label(Role::TextInput, "Username").click();
    harness.run_steps(2);
    harness
        .get_by_role_and_label(Role::TextInput, "Username")
        .type_text(user);
    harness.get_by_role_and_label(Role::PasswordInput, "Password").click();
    harness.run_steps(2);
    harness
        .get_by_role_and_label(Role::PasswordInput, "Password")
        .type_text(password);
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::Button, "Sign in").click();
    harness.run_steps(4);
}

#[test]
fn ada_signs_in_with_tide() {
    let mut harness = render();
    sign_in(&mut harness, "ada", "tide");
    assert_eq!(harness.state().me(), runs::model::Person::Ada);
    assert!(harness.state().prefs().user.is_some(), "signed in");
}

#[test]
fn a_wrong_password_is_said() {
    let mut harness = render();
    sign_in(&mut harness, "ada", "sea");
    assert!(harness.state().prefs().user.is_none());
    assert!(harness.state().sign_in_error.is_some());
}
