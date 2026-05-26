use hyprland::{
    data::{Clients, Layers},
    shared::HyprData,
};
use nix::{sys::signal::kill, unistd::Pid};

use crate::{
    APP_ID,
    client::{HyprlandClient, WaylandClient},
};

#[derive(Clone)]
pub struct AppState<T: WaylandClient> {
    pub clients: Vec<T>,
}

impl<T: WaylandClient> AppState<T> {
    pub fn get_num_clients(&self) -> usize {
        self.clients.len()
    }

    fn prune_dead_clients(&mut self) {
        self.clients.retain(|c| is_proc_alive(c.pid()));
    }
}

impl AppState<HyprlandClient> {
    pub fn new() -> hyprland::Result<Self> {
        let clients = Vec::new();
        let app = Self { clients };

        let clients = app.get_open_clients()?;

        Ok(Self { clients })
    }

    pub fn refresh(&mut self) -> hyprland::Result<()> {
        self.prune_dead_clients();
        self.clients.extend(self.get_open_clients()?);

        Ok(())
    }

    fn get_open_clients(&self) -> hyprland::Result<Vec<HyprlandClient>> {
        let windows = Clients::get()?;
        let windows = windows
            .iter()
            .filter(|c| {
                // Filter out gtkshutdown so the app doesn't kill itself
                c.class != APP_ID
                &&
                // Avoid overwriting existing clients
                !self.clients
                        .iter()
                        .any(|existing| existing.pid().as_raw() == c.pid)
            })
            .cloned()
            .map(HyprlandClient::from);

        let layers = Layers::get()?;
        let layers = layers
            .iter()
            .flat_map(|(_, display)| display.iter())
            .flat_map(|(_, layers)| layers.iter())
            .filter(|c| {
                !self
                    .clients
                    .iter()
                    .any(|existing| existing.pid().as_raw() == c.pid)
            })
            .cloned()
            .map(HyprlandClient::from);

        let mut clients = windows.chain(layers).collect::<Vec<HyprlandClient>>();
        clients.sort();
        clients.dedup();

        Ok(clients)
    }
}

fn is_proc_alive(pid: &Pid) -> bool {
    match kill(*pid, None) {
        Ok(_) => true,
        Err(nix::errno::Errno::EPERM) => true, // If we don't have permission to kill, assume proc is still running
        Err(_) => false,
    }
}
