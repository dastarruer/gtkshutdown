use hyprland::{data::Clients, shared::HyprData};

pub struct AppState {
    pub clients: Clients,
}

impl AppState {
    pub fn new() -> hyprland::Result<Self> {
        let clients = Clients::get()?;

        Ok(Self { clients })
    }
}
