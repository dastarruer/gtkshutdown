use crate::client_killer::{Client, WaylandBackend};

#[derive(Clone)]
pub(super) struct SwayBackend {}

impl WaylandBackend for SwayBackend {
    fn open_clients(&self) -> anyhow::Result<Vec<Client>> {
        todo!()
    }

    fn gracefully_close(&self, client: &super::Client) -> anyhow::Result<()> {
        todo!()
    }
}
