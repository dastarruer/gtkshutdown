use hyprland::data::{Client, LayerClient};
use nix::unistd::Pid;

pub trait WaylandClient {
    fn pid(&self) -> Pid;
    fn app_id(&self) -> &str;
    fn title(&self) -> Option<&str>;
}

#[derive(Clone)]
pub enum HyprlandClient {
    Window(Client),
    Layer(LayerClient),
}

impl WaylandClient for HyprlandClient {
    fn pid(&self) -> Pid {
        match self {
            HyprlandClient::Window(client) => Pid::from_raw(client.pid),
            HyprlandClient::Layer(layer) => Pid::from_raw(layer.pid),
        }
    }

    fn app_id(&self) -> &str {
        match self {
            HyprlandClient::Window(client) => &client.class,
            HyprlandClient::Layer(layer) => &layer.namespace, // Layer namespace is close enough to an app ID
        }
    }

    fn title(&self) -> Option<&str> {
        match self {
            HyprlandClient::Window(client) => Some(&client.title),
            HyprlandClient::Layer(_) => None, // Layers do not have titles
        }
    }
}
