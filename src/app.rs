use nix::{sys::signal::kill, unistd::Pid};

use crate::client_killer::{Backend, Client, WaylandBackend, WaylandClient};

#[derive(Clone)]
pub struct AppState {
    pub clients: Vec<Client>,
    backend: Backend,
}

impl AppState {
    pub fn new(backend: Backend) -> anyhow::Result<Self> {
        let clients = Vec::new();
        let clients = backend.get_open_clients(&clients)?;

        Ok(Self { clients, backend })
    }

    pub fn get_num_clients(&self) -> usize {
        self.clients.len()
    }

    pub fn refresh(&mut self) -> anyhow::Result<()> {
        self.prune_dead_clients();
        self.clients
            .extend(self.backend.get_open_clients(&self.clients)?);

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
