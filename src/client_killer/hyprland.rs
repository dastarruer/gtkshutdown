use std::fmt::Display;

use hyprland::{
    data::{Clients, LayerClient, Layers},
    dispatch::{Dispatch, DispatchType, WindowIdentifier},
    error::HyprError,
    shared::HyprData,
};
use nix::unistd::Pid;

use crate::{
    APP_ID,
    client_killer::{Client, KillStatus, WaylandBackend, WaylandClient},
};

#[derive(Clone, PartialEq, Eq, PartialOrd)]
enum HyprlandClientKind {
    Window,
    Layer,
}

#[derive(PartialEq, Eq, Clone)]
pub(super) struct HyprlandClient {
    pid: Pid,
    kind: HyprlandClientKind,
    app_id: String,
    title: Option<String>,
    status: KillStatus,
}

impl Display for HyprlandClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HyprlandClient {{ app_id: {}, pid: {} }}",
            self.app_id, self.pid
        )
    }
}

impl PartialOrd for HyprlandClient {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HyprlandClient {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.is_layer()
            .cmp(&other.is_layer()) // Sort non-layer clients lower
            .then_with(|| self.app_id().cmp(other.app_id())) // Sort clients by app_id
    }
}

impl From<hyprland::data::Client> for HyprlandClient {
    fn from(value: hyprland::data::Client) -> Self {
        Self {
            pid: Pid::from_raw(value.pid),
            title: Some(value.title.to_owned()),
            app_id: value.class.to_owned(),
            kind: HyprlandClientKind::Window,
            status: KillStatus::Alive,
        }
    }
}

impl From<LayerClient> for HyprlandClient {
    fn from(value: LayerClient) -> Self {
        Self {
            pid: Pid::from_raw(value.pid),
            title: None,
            app_id: value.namespace.to_owned(), // Layer namespace is close enough to an app ID
            kind: HyprlandClientKind::Layer,    // Layers do not have titles
            status: KillStatus::Alive,
        }
    }
}

impl WaylandClient for HyprlandClient {
    fn pid(&self) -> &Pid {
        &self.pid
    }

    fn app_id(&self) -> &str {
        &self.app_id
    }

    fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    fn is_layer(&self) -> bool {
        self.kind == HyprlandClientKind::Layer
    }

    fn status(&self) -> &KillStatus {
        &self.status
    }

    fn update_status(&mut self) {
        self.status = self.status.clone().update();
    }

    fn gracefully_close(&self) -> anyhow::Result<()> {
        let hyprlang_dispatch =
            DispatchType::CloseWindow(WindowIdentifier::ProcessId(self.pid().as_raw() as u32));

        let lua_args = format!(
            r#"hl.dsp.window.close({{ window = "pid:{}" }})"#,
            self.pid().as_raw()
        );

        // Equivalent of calling `hyprctl dispatch closewindow pid:<PID>`
        match Dispatch::call(hyprlang_dispatch) {
            Ok(_) => Ok(()),
            // If this happens, assume that the user is using hyprland lua
            Err(HyprError::NotOkDispatch(_)) => {
                log::debug!("Running: hyprctl dispatch {lua_args}");

                // Run hyprctl dispatch manually, since hyprland-rs doesn't support lua as of now
                let output = std::process::Command::new("hyprctl")
                    .args(["dispatch", &lua_args])
                    .output()?;

                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);

                if !output.status.success() {
                    log::error!(
                        "hyprctl dispatch failed (status {}): stdout={stdout} stderr={stderr}",
                        output.status
                    );
                } else {
                    log::debug!(
                        "hyprctl dispatch succeeded (status {}): stdout={stdout} stderr={stderr}",
                        output.status
                    );
                }

                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
}

#[derive(Clone)]
pub(super) struct HyprlandBackend {}

impl WaylandBackend for HyprlandBackend {
    fn get_open_clients(&self, existing_clients: &[Client]) -> anyhow::Result<Vec<Client>> {
        let windows = Clients::get()?;
        let windows = windows
            .iter()
            .filter(|c| {
                // Filter out gtkshutdown so the app doesn't kill itself
                c.class != APP_ID
                &&
                c.pid > 0 && // Skip negative PIDs to avoid nuking entire session
                // Avoid overwriting existing clients
                !existing_clients
                        .iter()
                        .any(|existing| existing.pid().as_raw() == c.pid)
            })
            .cloned()
            .map(Client::from);

        let layers = Layers::get()?;
        let layers = layers
            .iter()
            .flat_map(|(_, display)| display.iter())
            .flat_map(|(_, layers)| layers.iter())
            .filter(|c| {
                c.pid > 0
                    && !existing_clients
                        .iter()
                        .any(|existing| existing.pid().as_raw() == c.pid)
            })
            .cloned()
            .map(Client::from);

        let mut clients = windows.chain(layers).collect::<Vec<Client>>();
        clients.sort();
        clients.dedup();

        Ok(clients)
    }
}
