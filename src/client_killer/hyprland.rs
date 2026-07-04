use hyprland::{
    data::{Clients, Layers},
    dispatch::{Dispatch, DispatchType, WindowIdentifier},
    error::HyprError,
    shared::HyprData,
};

use crate::{
    APP_ID,
    client_killer::{Client, WaylandBackend},
};

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

    fn gracefully_close(&self, client: &super::Client) -> anyhow::Result<()> {
        let hyprlang_dispatch =
            DispatchType::CloseWindow(WindowIdentifier::ProcessId(client.pid().as_raw() as u32));

        let lua_args = format!(
            r#"hl.dsp.window.close({{ window = "pid:{}" }})"#,
            client.pid().as_raw()
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
