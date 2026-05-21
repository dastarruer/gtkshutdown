mod app;
mod client;
mod ui;

use std::cell::RefCell;
use std::rc::Rc;

use app::{AppState, ClientKiller};
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

    /// Set a command to be run after all apps have shut down. By default,
    /// gtkshutdown runs nothing after exiting.
    #[arg(short, long)]
    post_cmd: Option<String>,
}

fn main() -> glib::ExitCode {
    let args = Args::parse();
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(move |app| {
        let mut client_killer = ClientKiller::new();
        let state = Rc::new(RefCell::new(
            AppState::new().expect("Failed to get clients from Hyprland"),
        ));

        let mut ui = UiBuilder::new(app, &state.borrow());
        ui.window.present();

        let post_cmd = args.post_cmd.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
            state
                .borrow_mut()
                .refresh()
                .expect("Failed to get clients from Hyprland");

            if !args.dry_run {
                client_killer
                    .kill_clients(&state.borrow().clients)
                    .expect("Failed to kill process.");
            }

            ui.update(&state.borrow());

            if state.borrow().get_num_clients() == 0 {
                ui.window.close();

                if let Some(post_cmd) = &post_cmd {
                    let post_cmd = post_cmd.split(" ").collect::<Vec<&str>>();
                    let command = post_cmd
                        .first()
                        .expect("--post-cmd does not contain a valid command.");
                    let args = post_cmd.iter().skip(1).cloned().collect::<Vec<&str>>();

                    std::process::Command::new(command)
                        .args(args)
                        .spawn()
                        .expect("Unable to execute command in --post-cmd")
                        .wait()
                        .expect("Unable to execute command in --post-cmd");
                }

                return glib::ControlFlow::Break;
            }

            glib::ControlFlow::Continue
        });
    });

    // Overwrite gtk cli args to use our own
    app.run_with_args::<&str>(&[])
}
