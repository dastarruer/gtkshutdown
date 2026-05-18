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

pub fn kill_clients(state: &AppState) -> nix::Result<()> {
    for client in &state.clients {
        let pid = Pid::from_raw(client.pid);
        let signal = Signal::SIGTERM; // Use SIGTERM for graceful shutdown

        nix::sys::signal::kill(pid, signal)?;
    }

    Ok(())
}
