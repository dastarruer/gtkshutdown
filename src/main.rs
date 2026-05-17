mod ui;

use gtk4::prelude::*;
use gtk4::{Application, glib};
use ui::UiBuilder;

fn main() -> glib::ExitCode {
    let app = Application::builder()
        .application_id("io.github.dastarruer.gtkshutdown")
        .build();

    app.connect_activate(|app| {
        let ui = UiBuilder::new(app);

        ui.window.present();
    });

    app.run()
}
