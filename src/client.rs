use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

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
enum KillStatus {
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

pub struct ClientKiller {
    /// Represents seen processes to figure out when to send SIGKILL signal
    seen: HashMap<Pid, KillStatus>,
}

impl ClientKiller {
    pub fn new() -> Self {
        Self {
            seen: HashMap::new(),
        }
    }

    pub fn force_kill_clients<T: WaylandClient>(&self, clients: &[T]) -> nix::Result<()> {
        for client in clients {
            kill(client.pid(), Signal::SIGKILL)?;
        }

        Ok(())
    }

    pub fn kill_clients<T: WaylandClient>(&mut self, clients: &[T]) -> anyhow::Result<()> {
        for client in clients {
            self.kill_client(client)?;
        }

        // Remove processes that are dead
        self.seen.retain(|c, _| Self::is_proc_alive(c.to_owned()));
        Ok(())
    }

    fn kill_client<T: WaylandClient>(&mut self, client: &T) -> anyhow::Result<()> {
        let pid = client.pid();
        let status = self.seen.entry(pid).or_insert(KillStatus::Alive);

        if let Some(action) = status.poll() {
            match action {
                KillAction::Graceful => client.gracefully_close()?,
                KillAction::Sigkill => kill(pid, Signal::SIGKILL)?,
                KillAction::Sigterm => kill(pid, Signal::SIGTERM)?,
            }

            *status = status.clone().update();
        }

        Ok(())
    }

    fn is_proc_alive(pid: Pid) -> bool {
        match kill(pid, None) {
            Ok(_) => true,
            Err(nix::errno::Errno::EPERM) => true, // If we don't have permission to kill, assume proc is still running
            Err(_) => false,
        }
    }
}

pub trait WaylandClient {
    fn pid(&self) -> Pid;
    fn app_id(&self) -> &str;
    fn title(&self) -> Option<&str>;
    /// Meant to be used first before sending SIGTERM (and eventually SIGKILL)
    /// signal, so apps have a chance to gracefully exit.
    fn gracefully_close(&self) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub enum HyprlandClient {
    Window(Client),
    Layer(LayerClient),
}

impl WaylandClient for HyprlandClient {
    fn pid(&self) -> Pid {
        match self {
            HyprlandClient::Window(client) => Pid::from_raw(client.pid),
            HyprlandClient::Layer(layer) => Pid::from_raw(layer.pid),
        }
    }

    fn app_id(&self) -> &str {
        match self {
            HyprlandClient::Window(client) => &client.class,
            HyprlandClient::Layer(layer) => &layer.namespace, // Layer namespace is close enough to an app ID
        }
    }

    fn title(&self) -> Option<&str> {
        match self {
            HyprlandClient::Window(client) => Some(&client.title),
            HyprlandClient::Layer(_) => None, // Layers do not have titles
        }
    }

    fn gracefully_close(&self) -> anyhow::Result<()> {
        // Equivalent of calling `hyprctl dispatch closewindow pid:<PID>`
        Dispatch::call(DispatchType::CloseWindow(WindowIdentifier::ProcessId(
            self.pid().as_raw() as u32,
        )))?;
        Ok(())
    }
}
