pub mod hyprland;

use std::{
    fmt::Display,
    time::{Duration, Instant},
};

use anyhow::Context;

use nix::{
    sys::signal::{Signal, kill},
    unistd::Pid,
};

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

    pub fn force_kill_clients<T: WaylandClient>(&self, clients: &[T]) -> nix::Result<()> {
        for client in clients {
            kill(*client.pid(), Signal::SIGKILL)?;
        }

        Ok(())
    }

    pub fn kill_clients<T: WaylandClient + Display>(
        &mut self,
        clients: &mut [T],
    ) -> anyhow::Result<()> {
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

    fn kill_client<T: WaylandClient + Display>(&mut self, client: &mut T) -> anyhow::Result<()> {
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

pub trait WaylandClient {
    fn pid(&self) -> &Pid;
    fn app_id(&self) -> &str;
    fn title(&self) -> Option<&str>;
    fn is_layer(&self) -> bool;
    fn status(&self) -> &KillStatus;

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
