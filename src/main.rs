mod app;
mod client;
mod ui;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::Context;
use app::AppState;
use clap::Parser;
use client::{ClientKiller, HyprlandClient, WaylandClient};
use flexi_logger::{FileSpec, Logger};
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

impl Args {
    fn execute_post_cmd(&self) -> anyhow::Result<()> {
        if let Some(post_cmd) = &self.post_cmd {
            let post_cmd = post_cmd.split_whitespace().collect::<Vec<&str>>();

            let command = post_cmd.first().context("Unable to parse --post_cmd.")?;

            let args = post_cmd.iter().skip(1).cloned().collect::<Vec<&str>>();

            std::process::Command::new(command)
                .args(args)
                .spawn()
                .context("Unable to execute --post-cmd.")?;
        }

        Ok(())
    }
}

struct AppHandler<T: WaylandClient> {
    args: Args,
    state: Rc<RefCell<AppState<T>>>,
    ui: UiBuilder,
    client_killer: Rc<RefCell<ClientKiller>>,
}

impl AppHandler<HyprlandClient> {
    fn new(app: &Application, args: Args) -> anyhow::Result<Self> {
        let client_killer = Rc::new(RefCell::new(ClientKiller::new()));
        let state = Rc::new(RefCell::new(
            AppState::new().context("Failed to get clients from Hyprland.")?,
        ));
        let ui = UiBuilder::new(app, Rc::clone(&state), Rc::clone(&client_killer));

        Ok(Self {
            args,
            state,
            ui,
            client_killer,
        })
    }

    /// Execute a single tick of the app.
    ///
    /// # Returns
    ///
    /// - `true` if all clients have been closed.
    /// - `false` if clients are still open.
    fn tick(&mut self) -> anyhow::Result<bool> {
        log::info!("Refreshing client list...");
        self.state
            .borrow_mut()
            .refresh()
            .context("Failed to get clients from Hyprland")?;

        if !self.args.dry_run {
            log::info!("The killing begins! Killing open clients...");
            self.client_killer
                .borrow_mut()
                .kill_clients(&mut self.state.borrow_mut().clients)
                .context("Failed to kill process")?;
        }

        self.ui.update(&self.state.borrow());

        let is_complete = self.is_app_complete();
        log::trace!(
            "App is completed: {is_complete}, Number of clients: {}",
            self.state.borrow().get_num_clients()
        );

        Ok(is_complete)
    }

    fn is_app_complete(&self) -> bool {
        self.state.borrow().get_num_clients() == 0
    }
}

fn main() -> glib::ExitCode {
    let log_dir = std::env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").expect("HOME is not set");
            PathBuf::from(home).join(".local/state")
        })
        .join("gtkshutdown");

    let _logger = Logger::try_with_env()
        .expect("Value of RUST_LOG is malformed")
        .log_to_file(
            FileSpec::default()
                .directory(log_dir)
                .basename("gtkshutdown"),
        )
        .duplicate_to_stdout(flexi_logger::Duplicate::Trace)
        .rotate(
            flexi_logger::Criterion::Size(1000000),
            flexi_logger::Naming::Numbers,
            flexi_logger::Cleanup::KeepLogFiles(5),
        )
        .start()
        .expect("Logger failed to start");

    let args = Args::parse();
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(move |app| {
        let mut handler = AppHandler::new(app, args.clone()).unwrap_or_else(|e| {
            log::error!("Error starting app: {e}");
            std::process::exit(1);
        });

        handler.ui.window.present();
        log::debug!("Window created!");

        glib::timeout_add_local(std::time::Duration::from_millis(150), move || match handler
            .tick()
            .unwrap_or_else(|e| {
                log::error!("Error running app tick: {e}");
                std::process::exit(1);
            }) {
            true => {
                log::info!("All apps have been shut down, closing...");
                handler.ui.window.close();

                log::info!("Executing --post-cmd...");
                handler.args.execute_post_cmd().unwrap_or_else(|e| {
                    log::error!("Error executing --post-cmd: {e}");
                    std::process::exit(1);
                });

                glib::ControlFlow::Break
            }
            false => {
                log::debug!("All apps have not been shut down, moving on to the next tick...");
                glib::ControlFlow::Continue
            }
        });
    });

    // Overwrite gtk cli args to use our own
    app.run_with_args::<&str>(&[])
}
