mod app;
mod client;
mod ui;

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Context;
use app::AppState;
use clap::Parser;
use client::{ClientKiller, HyprlandClient, WaylandClient};
use gtk4::prelude::*;
use gtk4::{Application, glib};
use ui::UiBuilder;

pub const APP_ID: &str = "io.github.dastarruer.gtkshutdown";

#[derive(Parser, Debug, Clone)]
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

struct AppHandler<T: WaylandClient> {
    args: Args,
    state: Rc<RefCell<AppState<T>>>,
    ui: UiBuilder,
    client_killer: ClientKiller,
}

impl AppHandler<HyprlandClient> {
    fn new(app: &Application, args: Args) -> Self {
        let client_killer = ClientKiller::new();
        let state = Rc::new(RefCell::new(
            AppState::new().expect("Failed to get clients from Hyprland"),
        ));
        let ui = UiBuilder::new(app, &state.borrow());

        Self {
            args,
            state,
            ui,
            client_killer,
        }
    }

    /// Execute a single tick of the app.
    ///
    /// # Returns
    ///
    /// - `true` if all clients have been closed.
    /// - `false` if clients are still open.
    fn tick(&mut self) -> anyhow::Result<bool> {
        self.state
            .borrow_mut()
            .refresh()
            .context("Failed to get clients from Hyprland")?;

        if !self.args.dry_run {
            self.client_killer
                .kill_clients(&self.state.borrow().clients)
                .context("Failed to kill process.")?;
        }

        self.ui.update(&self.state.borrow());

        Ok(self.is_app_complete())
    }

    fn is_app_complete(&self) -> bool {
        self.state.borrow().get_num_clients() == 0
    }
}

fn main() -> glib::ExitCode {
    let args = Args::parse();
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(move |app| {
        let mut handler = AppHandler::new(app, args.clone());
        handler.ui.window.present();

        glib::timeout_add_local(
            std::time::Duration::from_millis(150),
            move || match handler.tick() {
                Ok(false) => glib::ControlFlow::Continue,
                Ok(true) => {
                    handler.ui.window.close();

                    if let Some(post_cmd) = &handler.args.post_cmd {
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

                    glib::ControlFlow::Break
                }
                Err(e) => panic!("Error while shutting down apps: {e}"),
            },
        );
    });

    // Overwrite gtk cli args to use our own
    app.run_with_args::<&str>(&[])
}
