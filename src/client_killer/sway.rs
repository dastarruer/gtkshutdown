use std::fmt::Display;

use nix::unistd::Pid;

use crate::client_killer::{Client, KillStatus, WaylandBackend, WaylandClient};

#[derive(PartialEq, Eq, Clone, PartialOrd, Ord)]
pub(super) struct SwayClient {
    pid: Pid,
    app_id: String,
    title: Option<String>,
    status: KillStatus,
}

impl Display for SwayClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SwayClient {{ app_id: {}, pid: {} }}",
            self.app_id, self.pid
        )
    }
}

impl WaylandClient for SwayClient {
    fn app_id(&self) -> &str {
        &self.app_id
    }

    fn pid(&self) -> &Pid {
        &self.pid
    }

    fn status(&self) -> &KillStatus {
        &self.status
    }

    fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    fn update_status(&mut self) {
        self.status = self.status.clone().update();
    }

    fn gracefully_close(&self) -> anyhow::Result<()> {
        todo!()
    }

    fn is_layer(&self) -> bool {
        todo!()
    }
}

#[derive(Clone)]
pub(super) struct SwayBackend {}

impl WaylandBackend for SwayBackend {
    fn get_open_clients(&self, existing_clients: &[Client]) -> anyhow::Result<Vec<Client>> {
        todo!()
    }
}
