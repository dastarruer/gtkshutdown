mod app;
mod ui;

use std::cell::RefCell;
use std::rc::Rc;

use app::{AppState, close_clients};
use gtk4::prelude::*;
use gtk4::{Application, glib};
use ui::UiBuilder;

pub const APP_ID: &str = "io.github.dastarruer.gtkshutdown";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(|app| {
        let state = Rc::new(RefCell::new(
            AppState::new().expect("Failed to get clients from Hyprland"),
        ));

        let mut ui = UiBuilder::new(app, &state.borrow());

        close_clients(&state.borrow());

        ui.window.present();

        glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
            state
                .borrow_mut()
                .refresh()
                .expect("Failed to get clients from Hyprland");
            ui.update(&state.borrow());

            glib::ControlFlow::Continue
        });
    });

    app.run()
}
