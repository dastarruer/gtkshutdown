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

const UNKNOWN_CLIENT_TITLE: &str = "(unknown)";

pub struct ClientKiller;

impl ClientKiller {
    pub fn force_kill_clients(clients: &[Client]) -> nix::Result<()> {
        for client in clients {
            kill(*client.pid(), Signal::SIGKILL)?;
        }

        Ok(())
    }

    pub fn kill_clients(
        backend: &dyn WaylandBackend,
        clients: &mut [Client],
    ) -> anyhow::Result<()> {
        for client in clients {
            log::trace!("Attempting to kill client {client}...");

            Self::kill_client(backend, client).with_context(|| {
                format!(
                    "Failed to kill client {} (pid: {})",
                    client.app_id(),
                    client.pid()
                )
            })?;
        }

        Ok(())
    }

    fn kill_client(backend: &dyn WaylandBackend, client: &mut Client) -> anyhow::Result<()> {
        let pid = *client.pid();
        let app_id = client.app_id();

        if client.is_layer() || client.unique_id().is_empty() {
            log::debug!("Sending SIGTERM to client {app_id}...");
            kill(pid, Signal::SIGTERM)?;

            return Ok(());
        } else {
            log::debug!("Requesting graceful close to client {app_id}...");
            backend.gracefully_close(client)?;
        }

        Ok(())
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Client {
    pid: Pid,
    // Used to quit apps. Since it may have different names across compositors,
    // it's called 'unique_id'
    unique_id: String,
    kind: ClientKind,
    /// The title of the app. For instance, an Anki window might have a `anki`
    /// app ID.
    app_id: String,
    /// The title of the open app page. For instance, an Anki window might have
    /// a `User 1 - Anki` title.
    title: Option<String>,
    instant_started: Instant,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Debug)]
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
    pub(self) fn new(
        pid: Pid,
        unique_id: String,
        kind: ClientKind,
        app_id: String,
        title: Option<String>,
    ) -> Self {
        Self {
            pid,
            unique_id,
            kind,
            app_id,
            title,
            instant_started: Instant::now(),
        }
    }

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

    pub fn unique_id(&self) -> &str {
        &self.unique_id
    }

    pub fn instant_started(&self) -> &Instant {
        &self.instant_started
    }

    /// Check if the client is asking the user to save their work. Note that
    /// there is no reliable way to detect save dialogs on Linux, so this is
    /// based on if the client is still open even after requesting it to
    /// gracefully exit.
    pub fn may_be_saving(&self) -> bool {
        self.instant_started().elapsed() > Duration::from_secs(5)
    }

    /// Check if the client is hanging if after sending a SIGTERM signal, the
    /// client still hasn't died.
    pub fn may_be_hanging(&self) -> bool {
        self.instant_started().elapsed() > Duration::from_secs(10)
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
                    hyprland::Backend::new()
                        .expect("hyprland backend should be successfully initialized"),
                ));
            }
            SWAY_STRING => {
                return Some(Box::new(
                    sway::Backend::new().expect("sway backend should be successfully initialized"),
                ));
            }
            _ => return None,
        }
    }

    None
}
