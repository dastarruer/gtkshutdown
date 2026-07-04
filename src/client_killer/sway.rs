use std::fmt::Display;

use nix::unistd::Pid;

use crate::client_killer::{KillStatus, WaylandClient};

#[derive(PartialEq, Eq)]
pub struct SwayClient {
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

    fn get_open_clients(existing_clients: &[Self]) -> anyhow::Result<Vec<Self>> {
        todo!()
    }

    fn gracefully_close(&self) -> anyhow::Result<()> {
        todo!()
    }

    fn is_layer(&self) -> bool {
        todo!()
    }
}
