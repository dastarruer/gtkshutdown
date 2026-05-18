mod app_state;
mod ui;

use std::cell::RefCell;
use std::rc::Rc;

use app_state::AppState;
use gtk4::prelude::*;
use gtk4::{Application, glib};
use ui::UiBuilder;

fn main() -> glib::ExitCode {
    let app = Application::builder()
        .application_id("io.github.dastarruer.gtkshutdown")
        .build();

    app.connect_activate(|app| {
        let state = Rc::new(RefCell::new(
            AppState::new().expect("Failed to get clients from Hyprland"),
        ));
        let ui = Rc::new(RefCell::new(UiBuilder::new(app, Rc::clone(&state))));
        let ui_clone = Rc::clone(&ui);

        glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
            state
                .borrow_mut()
                .refresh()
                .expect("Failed to get clients from Hyprland");
            ui_clone.borrow_mut().update(&state);

            glib::ControlFlow::Continue
        });

        ui.borrow().window.present();
    });

    app.run()
}
