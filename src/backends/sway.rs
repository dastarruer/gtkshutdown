use std::cell::RefCell;

use anyhow::bail;
use nix::unistd::Pid;
use swayipc::{Connection, Node, NodeType};

use crate::backends::{ClientKind, UNKNOWN_CLIENT_TITLE, WaylandBackend};

#[derive(Debug, PartialEq)]
struct Client {
    pid: i32,
    id: i64,
    app_id: String,
    name: Option<String>,
}

impl From<Client> for super::Client {
    fn from(value: Client) -> Self {
        Self::new(
            Pid::from_raw(value.pid),
            value.id.to_string(),
            ClientKind::Window,
            value.app_id,
            value.name, // Because app_id can sometimes be null, use app_id
                        // as title and title as app_id. simple really
        )
    }
}

impl TryFrom<Node> for Client {
    type Error = anyhow::Error;

    fn try_from(value: Node) -> Result<Self, Self::Error> {
        if !value.is_client() {
            bail!("expected client Node, got {value:?}")
        }

        let app_id = value
            .app_id
            .or_else(|| {
                value
                    .window_properties
                    .expect("node should have window properties")
                    .title
            })
            .unwrap_or_else(|| String::from(UNKNOWN_CLIENT_TITLE));
        let name = value.name.filter(|n| !n.is_empty());

        Ok(Self {
            pid: value.pid.expect("node should have a pid"),
            id: value.id,
            app_id,
            name,
        })
    }
}

trait NodeExt {
    fn is_client(&self) -> bool;
}

impl NodeExt for Node {
    fn is_client(&self) -> bool {
        self.pid.is_some() // Only clients have a pid
    }
}

struct RootNode(Node);

impl TryFrom<Node> for RootNode {
    type Error = anyhow::Error;

    fn try_from(value: Node) -> Result<Self, Self::Error> {
        let node_type = value.node_type;
        if node_type != NodeType::Root {
            bail!("expected root node type, got {node_type:?}")
        }

        Ok(RootNode(value))
    }
}

impl RootNode {
    fn clients(self) -> Vec<Client> {
        let mut clients = Vec::new();
        let mut queue = vec![&self.0];
        while let Some(child) = queue.pop() {
            queue.extend(&child.nodes);
            queue.extend(&child.floating_nodes);

            if child.is_client() {
                let client = Client::try_from(child.clone())
                    .expect("node should be successfully converted to SwayClient");
                clients.push(client);
            }
        }

        clients
    }
}

pub(super) struct Backend {
    connection: RefCell<Connection>,
}

impl Backend {
    pub(super) fn new() -> anyhow::Result<Self> {
        let connection = RefCell::new(Connection::new()?);
        Ok(Self { connection })
    }
}

impl WaylandBackend for Backend {
    fn open_clients(&self) -> anyhow::Result<Vec<super::Client>> {
        let root = RootNode::try_from(self.connection.borrow_mut().get_tree()?)?;

        // Sway doesn't expose layer information, so this just returns window
        // client types instead
        Ok(root
            .clients()
            .into_iter()
            .map(super::Client::from)
            .collect())
    }

    fn gracefully_close(&self, client: &super::Client) -> anyhow::Result<()> {
        let cmd = format!(r#"[pid={}] kill"#, client.pid());
        self.connection.borrow_mut().run_command(cmd)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_clients_from_tree() {
        let tree = include_str!("fixtures/sway/tree.json");
        let tree =
            serde_json::from_str::<Node>(tree).expect("tree should be successfully deserialized");
        let root = RootNode::try_from(tree).expect("tree should be a root Node");
        let mut clients = root.clients();

        let expected_urxvt = Client {
            pid: 23959,
            id: 5,
            app_id: String::from("urxvt"),
            name: Some(String::from("urxvt")),
        };
        let expected_termite = Client {
            pid: 25370,
            id: 6,
            app_id: String::from("termite"),
            name: None,
        };
        let mut expected = vec![expected_urxvt, expected_termite];

        expected.sort_by_key(|c| c.id);
        clients.sort_by_key(|c| c.id);

        pretty_assertions::assert_eq!(clients, expected);
    }
}
