use mango_ipc::{BlockingSocket, dispatch, query};
use nix::unistd::Pid;

use crate::backends::{ClientKind, WaylandBackend};

pub(super) struct Backend {
    socket: BlockingSocket,
}

impl From<mango_ipc::response::Client> for super::Client {
    fn from(value: mango_ipc::response::Client) -> Self {
        Self::new(
            Pid::from_raw(value.pid.cast_signed()),
            value.id.to_string(),
            ClientKind::Window,
            value.appid,
            Some(value.title),
        )
    }
}

impl Backend {
    pub(super) fn new() -> anyhow::Result<Self> {
        let socket = BlockingSocket::try_default()?;
        Ok(Self { socket })
    }
}

impl WaylandBackend for Backend {
    fn gracefully_close(&self, client: &super::Client) -> anyhow::Result<()> {
        self.socket
            .dispatch_with_client(dispatch::Killclient, client.unique_id.parse::<u32>().ok())?;
        Ok(())
    }

    fn open_clients(&self) -> anyhow::Result<Vec<super::Client>> {
        // mango ipc currently does not support getting layers.
        // I've opened a feature request here: https://github.com/mangowm/mango/issues/1360
        Ok(self
            .socket
            .get(query::AllClients)?
            .clients
            .into_iter()
            .map(super::Client::from)
            .collect())
    }
}
