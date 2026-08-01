use nix::{sys::signal::kill, unistd::Pid};

use crate::{
    APP_ID,
    client_killer::{Client, WaylandBackend},
};

pub struct AppState {
    pub clients: Vec<Client>,
    pub backend: Box<dyn WaylandBackend>,
}

impl AppState {
    pub fn new(backend: Box<dyn WaylandBackend>) -> anyhow::Result<Self> {
        let clients = backend.open_clients()?;

        Ok(Self { clients, backend })
    }

    pub fn get_num_clients(&self) -> usize {
        self.clients.len()
    }

    pub fn refresh(&mut self) -> anyhow::Result<()> {
        self.prune_dead_clients();
        let existing_clients = self.clients.clone();
        let open_clients = self.backend.open_clients()?;

        self.clients.extend(
            open_clients
                .iter()
                .filter(|c| {
                    // Filter out gtkshutdown so the app doesn't kill itself
                    c.app_id() != APP_ID
                        && !existing_clients
                            .iter()
                            .any(|existing| existing.pid() == c.pid())
                })
                .cloned(),
        );

        Ok(())
    }

    fn prune_dead_clients(&mut self) {
        self.clients.retain(|c| {
            let is_alive = is_proc_alive(c.pid());
            log::trace!("{} is alive: {is_alive}", c.app_id());

            is_alive
        });
    }
}

fn is_proc_alive(pid: &Pid) -> bool {
    match kill(*pid, None) {
        Ok(_) => true,
        Err(nix::errno::Errno::EPERM) => true, // If we don't have permission to kill, assume proc is still running
        Err(_) => false,
    }
}
