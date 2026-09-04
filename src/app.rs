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
    pub fn new(backend: Box<dyn WaylandBackend>) -> Self {
        Self {
            clients: Vec::new(), // This will repopulate itself automatically, since its length is already 0
            backend,
            to_be_killed: Vec::new(),
        }
    }

    pub const fn get_num_clients(&self) -> usize {
        self.clients.len()
    }

    pub fn refresh(&mut self) -> anyhow::Result<()> {
        let old_clients = self.clients.clone();

        self.clients = self
            .backend
            .open_clients()?
            .into_iter()
            .filter(|c| self.is_safe_to_kill(c))
            .collect();
        self.to_be_killed = old_clients
            .into_iter()
            .filter(|c| !self.clients.contains(c) && is_proc_alive(*c.pid()))
            .collect::<Vec<Client>>();
        Ok(())
    }

    fn is_safe_to_kill(&self, client: &Client) -> bool {
        client.app_id() != APP_ID
            && client.pid().as_raw() > 0
            && client.pid() != self.backend.compositor_id()
    }
}

fn is_proc_alive(pid: Pid) -> bool {
    match kill(pid, None) {
        // If we don't have permission to kill, assume proc is still running
        Ok(()) | Err(nix::errno::Errno::EPERM) => true,
        Err(_) => false,
    }
}
