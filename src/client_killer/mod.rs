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

use crate::client_killer::{hyprland::HyprlandBackend, sway::SwayBackend};

enum KillAction {
    Graceful,
    Sigterm,
    Sigkill,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
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

    pub fn kill_clients(
        &mut self,
        backend: &dyn WaylandBackend,
        clients: &mut [Client],
    ) -> anyhow::Result<()> {
        for client in clients {
            log::trace!("Attempting to kill client {client}...");

            self.kill_client(backend, client).with_context(|| {
                format!(
                    "Failed to kill client {} (pid: {})",
                    client.app_id(),
                    client.pid()
                )
            })?;
        }

        Ok(())
    }

    fn kill_client(
        &mut self,
        backend: &dyn WaylandBackend,
        client: &mut Client,
    ) -> anyhow::Result<()> {
        let pid = *client.pid();
        let status = client.status();

        let app_id = client.app_id();
        if let Some(action) = status.poll() {
            match action {
                KillAction::Graceful => {
                    if client.is_layer() || client.unique_id().is_empty() {
                        log::debug!("Sending SIGTERM to client {app_id}...");
                        kill(pid, Signal::SIGTERM)?;

                        return Ok(());
                    } else {
                        log::debug!("Requesting graceful close to client {app_id}...");
                        backend.gracefully_close(client)?;
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

#[derive(PartialEq, Eq, Clone)]
pub struct Client {
    pid: Pid,
    // Used to quit apps. Since it may have different names across compositors, 
    // it's called 'unique_id' 
    unique_id: String, 
    kind: ClientKind,
    app_id: String,
    title: Option<String>,
    status: KillStatus,
}

#[derive(Clone, PartialEq, Eq, PartialOrd)]
enum ClientKind {
    Window,
    Layer,
}

impl Display for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Client {{ app_id: {}, pid: {} }}", self.app_id, self.pid)
    }
}

impl PartialOrd for Client {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Client {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.is_layer()
            .cmp(&other.is_layer()) // Sort non-layer clients lower
            .then_with(|| self.app_id().cmp(other.app_id())) // Sort clients by app_id
    }
}

impl Client {
    pub fn pid(&self) -> &Pid {
        &self.pid
    }

    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn is_layer(&self) -> bool {
        self.kind == ClientKind::Layer
    }

    pub fn status(&self) -> &KillStatus {
        &self.status
    }

    pub fn unique_id(&self) -> &str {
        &self.unique_id
    }

    pub fn update_status(&mut self) {
        self.status = self.status.clone().update();
    }

    /// Check if the client is asking the user to save their work. Note that
    /// there is no reliable way to detect save dialogs on Linux, so this is
    /// based on if the client is still open even after requesting it to
    /// gracefully exit.
    pub fn may_be_saving(&self) -> bool {
        matches!(self.status(), KillStatus::GracefulSent(instant) if instant.elapsed() > Duration::from_secs(5))
    }

    /// Check if the client is hanging if after sending a SIGTERM signal, the
    /// client still hasn't died.
    pub fn may_be_hanging(&self) -> bool {
        matches!(self.status(), KillStatus::TermSent(instant) if instant.elapsed() > Duration::from_secs(3))
    }
}

pub trait WaylandBackend {
    /// Retrieves all currently-open clients.
    fn open_clients(&self) -> anyhow::Result<Vec<Client>>;

    /// Meant to be used first before sending SIGTERM (and eventually SIGKILL)
    /// signal, so apps have a chance to gracefully exit.
    fn gracefully_close(&self, client: &Client) -> anyhow::Result<()>;
}

/// Detects and returns the required backend by checking
/// `XDG_CURRENT_DESKTOP`.
pub fn detect_backend() -> Option<Box<dyn WaylandBackend>> {
    const HYPRLAND_STRING: &str = "Hyprland";
    const SWAY_STRING: &str = "sway";

    if let Ok(current_desktop) = &std::env::var("XDG_CURRENT_DESKTOP") {
        match current_desktop.as_str() {
            HYPRLAND_STRING => {
                return Some(Box::new(
                    HyprlandBackend::new()
                        .expect("hyprland backend should be successfully initialized"),
                ));
            }
            SWAY_STRING => return Some(Box::new(SwayBackend {})),
            _ => return None,
        }
    }

    None
}
