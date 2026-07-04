use crate::client_killer::{Client, WaylandBackend};

#[derive(Clone)]
pub(super) struct SwayBackend {}

impl WaylandBackend for SwayBackend {
    fn get_open_clients(&self, existing_clients: &[Client]) -> anyhow::Result<Vec<Client>> {
        todo!()
    }

    fn gracefully_close(&self, client: &super::Client) -> anyhow::Result<()> {
        todo!()
    }
}
