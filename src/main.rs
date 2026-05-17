use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, glib};
use gtk4::{self as gtk, Box, Label, Orientation};

struct UiBuilder {
    window: ApplicationWindow,
}

impl UiBuilder {
    fn new(app: &Application) -> Self {
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(1000)
            .default_height(200)
            .decorated(false)
            .resizable(false)
            .modal(true)
            .title("Hello world!!!!")
            .build();

        let root = Box::new(Orientation::Vertical, 8);

        root.append(&Self::build_header());

        window.set_child(Some(&root));
        Self { window }
    }

    fn build_header() -> Box {
        let header = Box::new(Orientation::Vertical, 0);

        let shutdown_header = Label::builder().build();
        shutdown_header.set_label("Shutting down...");

        header.append(&shutdown_header);

        header
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
