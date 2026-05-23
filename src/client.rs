use std::time::{Duration, Instant};

use hyprland::{
    data::{Client, LayerClient},
    dispatch::{Dispatch, DispatchType, WindowIdentifier},
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
            self.kill_client(client)?;
        }

        Ok(())
    }

    fn kill_client<T: WaylandClient>(&mut self, client: &mut T) -> anyhow::Result<()> {
        let pid = *client.pid();
        let status = &mut client.status();

        if let Some(action) = status.poll() {
            match action {
                KillAction::Graceful => {
                    if client.is_layer() {
                        // Layers do not need to be gracefully closed, and can just be SIGTERMed
                        kill(pid, Signal::SIGTERM)?;
                    } else {
                        client.gracefully_close()?;
                    }
                }
                KillAction::Sigterm => kill(pid, Signal::SIGTERM)?,
                KillAction::Sigkill => kill(pid, Signal::SIGKILL)?,
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
        // Equivalent of calling `hyprctl dispatch closewindow pid:<PID>`
        Dispatch::call(DispatchType::CloseWindow(WindowIdentifier::ProcessId(
            self.pid().as_raw() as u32,
        )))?;
        Ok(())
    }
}
