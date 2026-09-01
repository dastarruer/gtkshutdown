use nix::{sys::signal::kill, unistd::Pid};

use crate::{
    APP_ID,
    backends::{Client, WaylandBackend},
};

pub struct AppState {
    pub clients: Vec<Client>,
    /// Some apps may close their windows but still have leftover processes,
    /// and need to be `SIGTERM`'d.
    ///
    /// For instance, when exiting kitty, the window closes but there is
    /// still a `.kitty-wrapped` process that needs to be killed before kitty
    /// can be properly shut down.
    pub to_be_killed: Vec<Client>,
    pub backend: Box<dyn WaylandBackend>,
}

impl AppState {
    pub fn new(backend: Box<dyn WaylandBackend>) -> anyhow::Result<Self> {
        let clients = backend.open_clients()?;

        Ok(Self {
            clients,
            backend,
            to_be_killed: Vec::new(),
        })
    }

    pub fn get_num_clients(&self) -> usize {
        self.clients.len()
    }

    pub fn refresh(&mut self) -> anyhow::Result<()> {
        let old_clients = self.clients.clone();
        self.clients = self
            .backend
            .open_clients()?
            .into_iter()
            .filter(|c| c.app_id() != APP_ID && c.pid().as_raw() > 0)
            .collect();

        self.to_be_killed = old_clients
            .into_iter()
            .filter(|c| !self.clients.contains(c) && is_proc_alive(c.pid()))
            .collect::<Vec<Client>>();

        Ok(())
    }
}

fn is_proc_alive(pid: &Pid) -> bool {
    match kill(*pid, None) {
        Ok(_) => true,
        Err(nix::errno::Errno::EPERM) => true, // If we don't have permission to kill, assume proc is still running
        Err(_) => false,
    }
}
