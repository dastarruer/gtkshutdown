use hyprland::{
    data::{Client, LayerClient},
    dispatch::{Dispatch, DispatchType, WindowIdentifier},
};
use nix::unistd::Pid;

pub trait WaylandClient {
    fn pid(&self) -> Pid;
    fn app_id(&self) -> &str;
    fn title(&self) -> Option<&str>;
    /// Meant to be used first before sending SIGTERM (and eventually SIGKILL)
    /// signal, so apps have a chance to gracefully exit.
    fn gracefully_close(&self) -> anyhow::Result<()>;
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

    fn gracefully_close(&self) -> anyhow::Result<()> {
        // Equivalent of calling `hyprctl dispatch closewindow pid:<PID>`
        Dispatch::call(DispatchType::CloseWindow(WindowIdentifier::ProcessId(
            self.pid().as_raw() as u32,
        )))?;
        Ok(())
    }
}
