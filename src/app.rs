use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use hyprland::{
    data::{Client, Clients},
    shared::HyprData,
};
use nix::{sys::signal::Signal, unistd::Pid};

use crate::APP_ID;

pub struct AppState {
    pub clients: Vec<Client>,
}

impl AppState {
    pub fn new() -> hyprland::Result<Self> {
        let clients = Self::get_clients()?;

        Ok(Self { clients })
    }

    pub fn refresh(&mut self) -> hyprland::Result<()> {
        self.clients = Self::get_clients()?;
        Ok(())
    }

    pub fn get_num_clients(&self) -> usize {
        self.clients.len()
    }

    fn get_clients() -> hyprland::Result<Vec<Client>> {
        Ok(Clients::get()?
            .iter()
            // Filter out gtkshutdown so the app doesn't kill itself
            .filter(|c| c.class != APP_ID)
            .cloned()
            .collect())
    }
}

pub struct ClientKiller {
    /// Represents seen processes to figure out when to send SIGKILL signal
    seen: HashMap<Pid, Instant>,
}

impl ClientKiller {
    pub fn new() -> Self {
        Self {
            seen: HashMap::new(),
        }
    }

    pub fn kill_clients(&mut self, clients: &Vec<Client>) -> nix::Result<()> {
        for client in clients {
            let pid = Pid::from_raw(client.pid);
            self.send_shutdown_signal(pid)?;
        }

        Ok(())
    }

    fn send_shutdown_signal(&mut self, pid: Pid) -> nix::Result<()> {
        const SIGNAL: Signal = Signal::SIGTERM; // Use SIGTERM for graceful shutdown
        const SIGKILL_TIMEOUT: Duration = Duration::from_secs(5);

        match self.seen.get(&pid) {
            // After a certain amount of time, send a force kill signal (SIGKILL).
            Some(instant) if instant.elapsed() > SIGKILL_TIMEOUT => {
                nix::sys::signal::kill(pid, Signal::SIGKILL)?;
            }
            None => {
                self.seen.insert(pid, Instant::now());
                nix::sys::signal::kill(pid, SIGNAL)?;
            }
            _ => {}
        }

        Ok(())
    }
}
