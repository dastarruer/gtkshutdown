mod app;
mod ui;

use std::cell::RefCell;
use std::rc::Rc;

use app::{AppState, kill_clients};
use clap::Parser;
use gtk4::prelude::*;
use gtk4::{Application, glib};
use ui::UiBuilder;

pub const APP_ID: &str = "io.github.dastarruer.gtkshutdown";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Whether to only do a dry-run, where apps are not closed but the UI is
    /// still shown.
    #[arg(short, long, action)]
    dry_run: bool,
}

fn main() -> glib::ExitCode {
    let args = Args::parse();
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(move |app| {
        let state = Rc::new(RefCell::new(
            AppState::new().expect("Failed to get clients from Hyprland"),
        ));

        let mut ui = UiBuilder::new(app, &state.borrow());

        if !args.dry_run {
            kill_clients(&state.borrow()).unwrap_or_else(|e| panic!("Failed to kill process: {e}"));
        }

        ui.window.present();

        glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
            state
                .borrow_mut()
                .refresh()
                .expect("Failed to get clients from Hyprland");

            if !args.dry_run {
                kill_clients(&state.borrow())
                    .unwrap_or_else(|e| panic!("Failed to kill process: {e}"));
            }

            ui.update(&state.borrow());

            glib::ControlFlow::Continue
        });
    });

    // Overwrite gtk cli args to use our own
    app.run_with_args::<&str>(&[])
}
