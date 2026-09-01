use std::{
    collections::HashMap,
    env,
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::Path,
    time::Duration,
};

use anyhow::{Context, bail};
use nix::unistd::Pid;
use serde::Deserialize;

use crate::backends::{ClientKind, WaylandBackend};

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
struct WindowClient {
    pid: i32,
    address: String,
    class: String,
    title: String,
}

impl From<WindowClient> for super::Client {
    fn from(value: WindowClient) -> Self {
        Self::new(
            Pid::from_raw(value.pid),
            value.address,
            ClientKind::Window,
            value.class,
            Some(value.title),
        )
    }
}

#[derive(serde::Deserialize, Clone, Debug, PartialEq, Eq)]
struct Monitor {
    levels: HashMap<String, Vec<LayerClient>>,
}

#[derive(serde::Deserialize, Clone, Debug, PartialEq, Eq)]
struct LayerClient {
    pid: i32,
    address: String,
    namespace: String,
}

impl From<LayerClient> for super::Client {
    fn from(value: LayerClient) -> Self {
        Self::new(
            Pid::from_raw(value.pid),
            value.address,
            ClientKind::Layer,
            value.namespace,
            None,
        )
    }
}

#[derive(Deserialize)]
struct HyprlandStatus {
    #[serde(rename = "configProvider")]
    config_provider: String,
}

impl HyprlandStatus {
    fn is_using_lua(&self) -> bool {
        self.config_provider.as_str() == "lua"
    }
}

#[derive(Clone)]
pub(super) struct Backend {
    is_using_lua: bool,
}

impl Backend {
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
        let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR")
            .or_else(|_| std::env::var("UID").map(|uid| format!("/run/user/{uid}")))
            .expect("UID should exist");
        let hyprland_instance_signature = env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .expect("HYPRLAND_INSTANCE_SIGNATURE should exist");
        let socket_path =
            format!("{xdg_runtime_dir}/hypr/{hyprland_instance_signature}/.socket.sock");
        let socket = Path::new(&socket_path);
        UnixStream::connect(socket).context("unable to connect to hyprland IPC socket")
    }

    fn send_ipc_request(request: &str) -> anyhow::Result<String> {
        let timeout = Duration::from_secs(2);

        let mut stream = Self::ipc_stream()?;
        stream.set_read_timeout(Some(timeout))?;
        stream.set_write_timeout(Some(timeout))?;

        stream.write_all(request.as_bytes())?;
        stream.shutdown(std::net::Shutdown::Write)?;

        let mut response = String::new();
        stream.read_to_string(&mut response)?;

        if response.contains("error") || response.contains("unknown request") {
            bail!(response)
        }
        Ok(response)
    }

    fn open_windows() -> anyhow::Result<Vec<WindowClient>> {
        let response = Self::send_ipc_request("j/clients")?;
        Ok(serde_json::from_str::<Vec<WindowClient>>(&response)
            .expect("IPC windows response JSON should be successfully deserialized"))
    }

    fn open_layers() -> anyhow::Result<Vec<LayerClient>> {
        let response = Self::send_ipc_request("j/layers")?;
        Ok(Self::deserialize_layers_json(&response))
    }

    fn deserialize_layers_json(json: &str) -> Vec<LayerClient> {
        serde_json::from_str::<HashMap<String, Monitor>>(json)
            .expect("IPC layers response JSON should be successfully deserialized")
            .into_values()
            .flat_map(|m| m.levels.into_values())
            .flatten()
            .collect()
    }
}

impl WaylandBackend for Backend {
    fn open_clients(&self) -> anyhow::Result<Vec<super::Client>> {
        let windows = Self::open_windows()?;
        let windows = windows.into_iter().map(super::Client::from);

        let layers = Self::open_layers()?;
        let layers = layers.into_iter().map(super::Client::from);

        let mut clients = windows.chain(layers).collect::<Vec<super::Client>>();
        clients.sort();
        clients.dedup();

        Ok(clients)
    }

    fn gracefully_close(&self, client: &super::Client) -> anyhow::Result<()> {
        let address = client.unique_id();
        let cmd = if self.is_using_lua {
            format!("dispatch hl.dsp.window.close({{ window = \"address:{address}\" }})")
        } else {
            format!("dispatch closewindow address:{address}")
        };
        Self::send_ipc_request(&cmd)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn deserialize_window_client() {
        let json = indoc! {r#"
            [{
                "address": "0x6071d717d3c0",
                "floating": false,
                "monitor": 1,
                "class": "window",
                "title": "~",
                "initialClass": "window",
                "initialTitle": "window",
                "pid": 3441,
                "xwayland": false,
                "pinned": false,
                "pinFullscreened": false,
                "fullscreen": 0,
                "fullscreenClient": 0,
                "fullscreenHandler": "scrolling",
                "allowedOverFullscreen": false,
                "grouped": [],
                "tags": [],
                "swallowing": "0x0",
                "focusHistoryID": 1,
                "inhibitingIdle": false,
                "xdgTag": "",
                "xdgDescription": "",
                "contentType": "none",
                "tearingHint": false,
                "stableId": "1800001b"
            }]
        "#};
        let client = serde_json::from_str::<Vec<WindowClient>>(json)
            .expect("test JSON should be successfully deserialized");
        let expected = vec![WindowClient {
            pid: 3441,
            address: String::from("0x6071d717d3c0"),
            title: String::from("~"),
            class: String::from("window"),
        }];

        pretty_assertions::assert_eq!(client, expected);
    }

    #[test]
    fn deserialize_layer_clients() {
        let json = indoc! {r#"
            {
            "monitor_1": {
                "levels": {
                    "0": [
                            {
                                "address": "0x6071d7158a90",
                                "x": 1920,
                                "y": 0,
                                "w": 1920,
                                "h": 1080,
                                "alpha": 1,
                                "namespace": "layer",
                                "pid": 3442
                            }
                    ],
                    "1": [
            ],
                    "2": [
                            {
                                "address": "0x6071d7158a90",
                                "x": 1920,
                                "y": 0,
                                "w": 1920,
                                "h": 48,
                                "alpha": 1,
                                "namespace": "layer",
                                "pid": 3442
                            }
                    ],
                    "3": [
            ]
                }
            },"monitor_2": {
                "levels": {

                    "0": [
                            {
                                "address": "0x6071d7158a90",
                                "x": 0,
                                "y": 0,
                                "w": 1920,
                                "h": 1080,
                                "alpha": 1,
                                "namespace": "layer",
                                "pid": 3442
                            }
                    ],
                    "1": [
            ],
                    "2": [
                            {
                                "address": "0x6071d7158a90",
                                "x": 0,
                                "y": 0,
                                "w": 1920,
                                "h": 48,
                                "alpha": 0,
                                "namespace": "layer",
                                "pid": 3442
                            }
                    ],
                    "3": [
            ]
                }
            }
            }
        "#};
        let monitors = serde_json::from_str::<HashMap<String, Monitor>>(json)
            .expect("test JSON should be successfully deserialized");

        let expected_client = LayerClient {
            pid: 3442,
            address: String::from("0x6071d7158a90"),
            namespace: String::from("layer"),
        };

        let mut monitor_1_levels = HashMap::new();
        monitor_1_levels.insert(String::from("0"), vec![expected_client.clone()]);
        monitor_1_levels.insert(String::from("1"), vec![]);
        monitor_1_levels.insert(String::from("2"), vec![expected_client.clone()]);
        monitor_1_levels.insert(String::from("3"), vec![]);

        let mut monitor_2_levels = HashMap::new();
        monitor_2_levels.insert(String::from("0"), vec![expected_client.clone()]);
        monitor_2_levels.insert(String::from("1"), vec![]);
        monitor_2_levels.insert(String::from("2"), vec![expected_client.clone()]);
        monitor_2_levels.insert(String::from("3"), vec![]);

        let mut expected = HashMap::new();
        expected.insert(
            String::from("monitor_1"),
            Monitor {
                levels: monitor_1_levels,
            },
        );
        expected.insert(
            String::from("monitor_2"),
            Monitor {
                levels: monitor_2_levels,
            },
        );

        pretty_assertions::assert_eq!(monitors, expected);

        let mut layers = Backend::deserialize_layers_json(json);
        layers.sort_by_key(|c| (c.pid, c.namespace.clone()));
        let expected = vec![
            expected_client.clone(),
            expected_client.clone(),
            expected_client.clone(),
            expected_client,
        ];

        pretty_assertions::assert_eq!(layers, expected);
    }
}
