use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, glib};
use gtk4::{self as gtk};

struct UiBuilder {
    window: ApplicationWindow,
}

impl UiBuilder {
    fn new(app: &Application) -> Self {
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(320)
            .default_height(200)
            .decorated(false)
            .resizable(false)
            .modal(true)
            .title("Hello world!!!!")
            .build();

        Self { window }
    }
}

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
