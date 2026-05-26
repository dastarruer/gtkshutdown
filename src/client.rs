use std::time::{Duration, Instant};

use anyhow::Context;
use hyprland::{
    data::{Client, LayerClient},
    dispatch::{Dispatch, DispatchType, WindowIdentifier},
    error::HyprError,
};
use nix::{
    sys::signal::{Signal, kill},
    unistd::Pid,
};

enum KillAction {
    Graceful,
    Sigterm,
    Sigkill,
}

#[derive(Clone)]
pub enum KillStatus {
    Alive,
    GracefulSent(Instant),
    TermSent(Instant),
    KillSent,
}

impl KillStatus {
    fn update(self) -> Self {
        match self {
            Self::Alive => Self::GracefulSent(Instant::now()),
            Self::GracefulSent(_) => Self::TermSent(Instant::now()),
            Self::TermSent(_) => Self::KillSent,
            Self::KillSent => Self::KillSent,
        }
    }

    fn poll(&self) -> Option<KillAction> {
        const SIGTERM_TIMEOUT: Duration = Duration::from_secs(15);
        const SIGKILL_TIMEOUT: Duration = Duration::from_secs(30);

        match self {
            Self::Alive => Some(KillAction::Graceful),
            Self::GracefulSent(instant) if instant.elapsed() > SIGTERM_TIMEOUT => {
                Some(KillAction::Sigterm)
            }
            Self::TermSent(instant) if instant.elapsed() > SIGKILL_TIMEOUT => {
                Some(KillAction::Sigkill)
            }
            _ => None,
        }
    }
}

pub struct ClientKiller {}

impl ClientKiller {
    pub fn new() -> Self {
        Self {}
    }

    pub fn force_kill_clients<T: WaylandClient>(&self, clients: &[T]) -> nix::Result<()> {
        for client in clients {
            kill(*client.pid(), Signal::SIGKILL)?;
        }

        Ok(())
    }

    pub fn kill_clients<T: WaylandClient>(&mut self, clients: &mut [T]) -> anyhow::Result<()> {
        for client in clients {
            self.kill_client(client).with_context(|| {
                format!(
                    "Failed to kill client {} (pid: {})",
                    client.app_id(),
                    client.pid()
                )
            })?;
        }

        Ok(())
    }

    fn kill_client<T: WaylandClient>(&mut self, client: &mut T) -> anyhow::Result<()> {
        let pid = *client.pid();
        let status = &mut client.status();

        let app_id = client.app_id();
        if let Some(action) = status.poll() {
            match action {
                KillAction::Graceful => {
                    if client.is_layer() {
                        log::debug!("Client {app_id} is a layer, sending SIGTERM...");
                        kill(pid, Signal::SIGTERM)?;
                    } else {
                        log::debug!("Requesting graceful close to client {app_id}...");
                        client.gracefully_close()?;
                    }
                }
                KillAction::Sigterm => {
                    log::warn!("Sending SIGTERM to client {app_id}...");
                    kill(pid, Signal::SIGTERM)?
                }
                KillAction::Sigkill => {
                    log::warn!("Sending SIGKILL to client {app_id}...");
                    kill(pid, Signal::SIGKILL)?;
                }
            }

            *status = &status.clone().update();
        }

        Ok(())
    }
}

pub trait WaylandClient {
    fn pid(&self) -> &Pid;
    fn app_id(&self) -> &str;
    fn title(&self) -> Option<&str>;
    fn is_layer(&self) -> bool;
    fn status(&self) -> &KillStatus;
    /// Meant to be used first before sending SIGTERM (and eventually SIGKILL)
    /// signal, so apps have a chance to gracefully exit.
    fn gracefully_close(&self) -> anyhow::Result<()>;
}

#[derive(Clone, PartialEq)]
enum HyprlandClientKind {
    Window,
    Layer,
}

pub struct HyprlandClient {
    pid: Pid,
    kind: HyprlandClientKind,
    app_id: String,
    title: Option<String>,
    status: KillStatus,
}

impl From<Client> for HyprlandClient {
    fn from(value: Client) -> Self {
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

    fn gracefully_close(&self) -> anyhow::Result<()> {
        let hyprlang_dispatch =
            DispatchType::CloseWindow(WindowIdentifier::ProcessId(self.pid().as_raw() as u32));

        let lua_args = format!(
            "hl.dsp.window.close({{ address = \"pid:{}\" }})",
            self.pid().as_raw()
        );

        // Equivalent of calling `hyprctl dispatch closewindow pid:<PID>`
        match Dispatch::call(hyprlang_dispatch) {
            Ok(_) => Ok(()),
            // If this happens, assume that the user is using hyprland lua
            Err(HyprError::NotOkDispatch(_)) => {
                // Run hyprctl dispatch manually, since hyprland-rs doesn't support lua as of now
                std::process::Command::new("hyprctl")
                    .args(["dispatch", &lua_args])
                    .output()?;
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
}
