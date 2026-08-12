use std::{
    collections::HashMap,
    env,
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::Path,
};

use anyhow::Context;
use nix::unistd::Pid;
use serde::Deserialize;

use crate::client_killer::{Client, ClientKind, KillStatus, WaylandBackend};

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
struct HyprlandWindowClient {
    pid: i32,
    class: String,
    title: String,
}

impl From<HyprlandWindowClient> for Client {
    fn from(value: HyprlandWindowClient) -> Self {
        Self {
            pid: Pid::from_raw(value.pid),
            app_id: value.class,
            title: Some(value.title),
            kind: ClientKind::Window,
            status: KillStatus::Alive,
        }
    }
}

#[derive(serde::Deserialize, Clone, Debug, PartialEq, Eq)]
struct Monitor {
    levels: HashMap<String, Vec<HyprlandLayerClient>>,
}

#[derive(serde::Deserialize, Clone, Debug, PartialEq, Eq)]
struct HyprlandLayerClient {
    pid: i32,
    namespace: String,
}

impl From<HyprlandLayerClient> for Client {
    fn from(value: HyprlandLayerClient) -> Self {
        Self {
            pid: Pid::from_raw(value.pid),
            app_id: value.namespace,
            title: None,
            kind: ClientKind::Layer,
            status: KillStatus::Alive,
        }
    }
}

#[derive(Deserialize)]
struct HyprlandStatus {
    #[serde(rename = "configProvider")]
    config_provider: String,
}

impl HyprlandStatus {
    fn is_using_lua(&self) -> bool {
        match self.config_provider.as_str() {
            "lua" => true,
            _ => false, // very in-depth checks i know
        }
    }
}

#[derive(Clone)]
pub(super) struct HyprlandBackend {
    is_using_lua: bool,
}

impl HyprlandBackend {
    pub(super) fn new() -> anyhow::Result<Self> {
        let status = Self::status()?;
        let is_using_lua = status.is_using_lua();

        Ok(Self { is_using_lua })
    }

    /// Returns the result of `hyprctl status`.
    fn status() -> anyhow::Result<HyprlandStatus> {
        let response = Self::send_ipc_request("j/status")?;
        Ok(serde_json::from_str::<HyprlandStatus>(&response)
            .expect("IPC status JSON should be successfully deserialized"))
    }

    /// Constructs and returns a stream connection to the hyprland socket. This
    /// method assumes that `HYPRLAND_INSTANCE_SIGNATURE` and `XDG_RUNTIME_DIR`
    /// exists at runtime.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// - Socket connection was unsuccessful
    fn ipc_stream() -> anyhow::Result<UnixStream> {
        let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR should exist");
        let hyprland_instance_signature = env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .expect("HYPRLAND_INSTANCE_SIGNATURE should exist");
        let socket_path =
            format!("{xdg_runtime_dir}/hypr/{hyprland_instance_signature}/.socket.sock");
        let socket = Path::new(&socket_path);
        UnixStream::connect(socket).context("unable to connect to hyprland IPC socket")
    }

    fn send_ipc_request(request: &str) -> anyhow::Result<String> {
        let mut stream = Self::ipc_stream()?;
        stream.write_all(request.as_bytes())?;
        stream.shutdown(std::net::Shutdown::Write)?;

        let mut response = String::new();
        stream.read_to_string(&mut response)?;

        Ok(response)
    }

    fn open_windows() -> anyhow::Result<Vec<HyprlandWindowClient>> {
        let response = Self::send_ipc_request("j/clients")?;
        Ok(serde_json::from_str::<Vec<HyprlandWindowClient>>(&response)
            .expect("IPC windows response JSON should be successfully deserialized"))
    }

    fn open_layers() -> anyhow::Result<Vec<HyprlandLayerClient>> {
        let response = Self::send_ipc_request("j/layers")?;
        Ok(Self::deserialize_layers_json(&response))
    }

    fn deserialize_layers_json(json: &str) -> Vec<HyprlandLayerClient> {
        serde_json::from_str::<HashMap<String, Monitor>>(json)
            .expect("IPC layers response JSON should be successfully deserialized")
            .into_values()
            .flat_map(|m| m.levels.into_values())
            .flatten()
            .collect()
    }
}

impl WaylandBackend for HyprlandBackend {
    fn open_clients(&self) -> anyhow::Result<Vec<Client>> {
        let windows = Self::open_windows()?;
        let windows = windows
            .iter()
            .filter(|c| {
                // Skip negative PIDs to avoid nuking entire session
                c.pid > 0
            })
            .cloned()
            .map(Client::from);

        let layers = Self::open_layers()?;
        let layers = layers
            .iter()
            .filter(|c| c.pid > 0)
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
