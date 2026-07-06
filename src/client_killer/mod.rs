pub mod hyprland;
pub mod sway;

use std::{
    fmt::Display,
    time::{Duration, Instant},
};

use anyhow::Context;

use nix::{
    sys::signal::{Signal, kill},
    unistd::Pid,
};

use crate::client_killer::{hyprland::HyprlandClient, sway::SwayClient};

enum KillAction {
    Graceful,
    Sigterm,
    Sigkill,
}

#[derive(Clone, PartialEq, Eq, PartialOrd)]
pub enum KillStatus {
    Alive,
    GracefulSent(Instant),
    TermSent(Instant),
    KillSent,
}

impl Display for KillStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Alive => write!(f, "Alive"),
            Self::GracefulSent(t) => {
                write!(f, "GracefulSent ({:.1}s ago)", t.elapsed().as_secs_f32())
            }
            Self::TermSent(t) => write!(f, "TermSent ({:.1}s ago)", t.elapsed().as_secs_f32()),
            Self::KillSent => write!(f, "KillSent"),
        }
    }
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

    pub fn force_kill_clients(&self, clients: &[Client]) -> nix::Result<()> {
        for client in clients {
            kill(*client.pid(), Signal::SIGKILL)?;
        }

        Ok(())
    }

    pub fn kill_clients(&mut self, clients: &mut [Client]) -> anyhow::Result<()> {
        for client in clients {
            log::trace!("Attempting to kill client {client}...");

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

    fn kill_client(&mut self, client: &mut Client) -> anyhow::Result<()> {
        let pid = *client.pid();
        let status = client.status();

        let app_id = client.app_id();
        if let Some(action) = status.poll() {
            match action {
                KillAction::Graceful => {
                    if client.is_layer() {
                        log::debug!("Client {app_id} is a layer, sending SIGTERM...");
                        kill(pid, Signal::SIGTERM)?;

                        return Ok(());
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

            log::trace!("Updating client {client} status...");
            client.update_status();
            log::trace!("New client status: {}", client.status());
        }

        Ok(())
    }
}

pub trait WaylandClient: Sized {
    fn pid(&self) -> &Pid;
    fn app_id(&self) -> &str;
    fn title(&self) -> Option<&str>;
    fn is_layer(&self) -> bool;
    fn status(&self) -> &KillStatus;
    fn get_open_clients(existing_clients: &[Self]) -> anyhow::Result<Vec<Self>>;

    /// Meant to be used first before sending SIGTERM (and eventually SIGKILL)
    /// signal, so apps have a chance to gracefully exit.
    fn gracefully_close(&self) -> anyhow::Result<()>;
    fn update_status(&mut self);

    /// Check if the client is asking the user to save their work. Note that
    /// there is no reliable way to detect save dialogs on Linux, so this is
    /// based on if the client is still open even after requesting it to
    /// gracefully exit.
    fn may_be_saving(&self) -> bool {
        matches!(self.status(), KillStatus::GracefulSent(instant) if instant.elapsed() > Duration::from_secs(5))
    }

    /// Check if the client is hanging if after sending a SIGTERM signal, the
    /// client still hasn't died.
    fn may_be_hanging(&self) -> bool {
        matches!(self.status(), KillStatus::TermSent(instant) if instant.elapsed() > Duration::from_secs(3))
    }
}

#[derive(Clone)]
pub struct Client {
    inner: ClientKind,
}

#[derive(Clone)]
enum ClientKind {
    Hyprland(HyprlandClient),
    Sway(SwayClient),
}

impl std::fmt::Display for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.client() {
            ClientKind::Hyprland(client) => write!(f, "{client}"),
            ClientKind::Sway(client) => write!(f, "{client}"),
        }
    }
}

impl Client {
    fn client(&self) -> &ClientKind {
        &self.inner
    }

    fn client_mut(&mut self) -> &mut ClientKind {
        &mut self.inner
    }
}

impl WaylandClient for Client {
    fn pid(&self) -> &Pid {
        match self.client() {
            ClientKind::Hyprland(client) => client.pid(),
            ClientKind::Sway(client) => client.pid(),
        }
    }

    fn app_id(&self) -> &str {
        match self.client() {
            ClientKind::Hyprland(client) => client.app_id(),
            ClientKind::Sway(client) => client.app_id(),
        }
    }

    fn title(&self) -> Option<&str> {
        match self.client() {
            ClientKind::Hyprland(client) => client.title(),
            ClientKind::Sway(client) => client.title(),
        }
    }

    fn is_layer(&self) -> bool {
        match self.client() {
            ClientKind::Hyprland(client) => client.is_layer(),
            ClientKind::Sway(client) => client.is_layer(),
        }
    }

    fn status(&self) -> &KillStatus {
        match self.client() {
            ClientKind::Hyprland(client) => client.status(),
            ClientKind::Sway(client) => client.status(),
        }
    }

    // fn get_open_clients(existing_clients: &[Self]) -> anyhow::Result<Vec<Self>> {
    //     match existing_clients.first() {
    //         Some(ClientKind::Hyprland(_)) => {
    //             let existing: Vec<_> = existing_clients
    //                 .iter()
    //                 .map(|client| match client {
    //                     ClientKind::Hyprland(client) => client,
    //                     _ => unreachable!("mixed client types"),
    //                 })
    //                 .cloned()
    //                 .collect();

    //             Ok(HyprlandClient::get_open_clients(&existing)?
    //                 .into_iter()
    //                 .map(ClientKind::Hyprland)
    //                 .collect())
    //         }
    //         Some(ClientKind::Sway(_)) => {
    //             let existing: Vec<_> = existing_clients
    //                 .iter()
    //                 .map(|client| match client {
    //                     ClientKind::Sway(client) => client,
    //                     _ => unreachable!("mixed client types"),
    //                 })
    //                 .cloned()
    //                 .collect();

    //             Ok(SwayClient::get_open_clients(&existing)?
    //                 .into_iter()
    //                 .map(ClientKind::Sway)
    //                 .collect())
    //         }
    //         None => {
    //             todo!("Need some way to determine which backend to query.");
    //         }
    //     }
    // }

    fn gracefully_close(&self) -> anyhow::Result<()> {
        match self.client() {
            ClientKind::Hyprland(client) => client.gracefully_close(),
            ClientKind::Sway(client) => client.gracefully_close(),
        }
    }

    fn update_status(&mut self) {
        match self.client_mut() {
            ClientKind::Hyprland(client) => client.update_status(),
            ClientKind::Sway(client) => client.update_status(),
        }
    }
}
