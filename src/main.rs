mod app_state;
mod ui;

use app_state::AppState;
use gtk4::prelude::*;
use gtk4::{Application, glib};
use ui::UiBuilder;

fn main() -> glib::ExitCode {
    let app = Application::builder()
        .application_id("io.github.dastarruer.gtkshutdown")
        .build();

    app.connect_activate(|app| {
        let state = AppState::new().expect("Failed to get clients from Hyprland");
        let ui = UiBuilder::new(app, state);

        ui.window.present();
    });

    app.run()
}
