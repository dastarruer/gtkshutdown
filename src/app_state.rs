use hyprland::{data::Clients, shared::HyprData};

pub struct AppState {
    pub clients: Clients,
}

impl AppState {
    pub fn new() -> hyprland::Result<Self> {
        let clients = Clients::get()?;

        Ok(Self { clients })
    }

    pub fn refresh(&mut self) -> hyprland::Result<()> {
        self.clients = Clients::get()?;
        Ok(())
    }
}
