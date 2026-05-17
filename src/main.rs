use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, glib};
use gtk4::{self as gtk, Button};

fn main() -> glib::ExitCode {
    let app = Application::builder()
        .application_id("io.github.dastarruer.gtkshutdown")
        .build();

    app.connect_activate(|app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(320)
            .default_height(200)
            .decorated(false)
            .resizable(false)
            .modal(true)
            .title("Hello world!!!!")
            .build();

        let button = Button::with_label("Click me");
        button.connect_clicked(|_| {
            eprintln!("Clicked");
        });
        window.set_child(Some(&button));

        window.present();
    });

    app.run()
}
