use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use hyprland::{
    data::{Clients, Layers},
    shared::HyprData,
};
use nix::{sys::signal::Signal, unistd::Pid};

use crate::{
    APP_ID,
    client::{HyprlandClient, WaylandClient},
};

pub struct AppState<T: WaylandClient> {
    pub clients: Vec<T>,
}

impl<T: WaylandClient> AppState<T> {
    pub fn get_num_clients(&self) -> usize {
        self.clients.len()
    }
}

impl AppState<HyprlandClient> {
    pub fn new() -> hyprland::Result<Self> {
        let clients = Self::get_open_clients()?;

        Ok(Self { clients })
    }

    pub fn refresh(&mut self) -> hyprland::Result<()> {
        self.clients = Self::get_open_clients()?;
        Ok(())
    }

    fn get_open_clients() -> hyprland::Result<Vec<HyprlandClient>> {
        let windows = Clients::get()?;
        let windows = windows
            .iter()
            // Filter out gtkshutdown so the app doesn't kill itself
            .filter(|c| c.class != APP_ID)
            .cloned()
            .map(HyprlandClient::Window);

        let layers = Layers::get()?;
        let layers = layers
            .iter()
            .flat_map(|(_, display)| display.iter())
            .flat_map(|(_, layers)| layers.iter())
            .cloned()
            .map(HyprlandClient::Layer);

        let mut clients = windows.chain(layers).collect::<Vec<HyprlandClient>>();
        clients.sort_by_key(|c| {
            // To place layers at the end of the vec, making them appear at the
            // bottom of the app list
            let is_layer = matches!(c, HyprlandClient::Layer(_));

            // Also sort by app id so clients don't jump all over the place in
            // the vec
            //
            // Could sort by pid to avoid cloning but this is negligible
            // at best, so don't care
            let app_id = c.app_id().to_owned();

            (is_layer, app_id)
        });

        Ok(clients)
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

    pub fn kill_clients<T: WaylandClient>(&mut self, clients: &Vec<T>) -> anyhow::Result<()> {
        for client in clients {
            self.kill_client(client)?;
        }

        Ok(())
    }

    fn kill_client<T: WaylandClient>(&mut self, client: &T) -> anyhow::Result<()> {
        const SIGTERM_TIMEOUT: Duration = Duration::from_secs(5);
        const SIGKILL_TIMEOUT: Duration = Duration::from_secs(30);

        let pid = client.pid();
        match self.seen.get(&pid) {
            Some(instant) if instant.elapsed() > SIGTERM_TIMEOUT => {
                nix::sys::signal::kill(pid, Signal::SIGTERM)?;
            }
            Some(instant) if instant.elapsed() > SIGKILL_TIMEOUT => {
                nix::sys::signal::kill(pid, Signal::SIGKILL)?;
            }
            None => {
                self.seen.insert(pid, Instant::now());
                client.gracefully_close()?;
            }
            _ => {}
        }

        Ok(())
    }
}
