use hyprland::{
    data::{Client, Clients},
    dispatch::{Dispatch, DispatchType, WindowIdentifier},
    shared::HyprData,
};

use crate::APP_ID;

pub struct AppState {
    pub clients: Vec<Client>,
}

impl AppState {
    pub fn new() -> hyprland::Result<Self> {
        let clients = Self::get_clients()?;

        Ok(Self { clients })
    }

    pub fn refresh(&mut self) -> hyprland::Result<()> {
        self.clients = Self::get_clients()?;
        Ok(())
    }

    pub fn get_num_clients(&self) -> usize {
        self.clients.len()
    }

    fn get_clients() -> hyprland::Result<Vec<Client>> {
        Ok(Clients::get()?
            .iter()
            // Filter out gtkshutdown so the app doesn't kill itself
            .filter(|c| c.class != APP_ID)
            .cloned()
            .collect())
    }
}

pub fn close_clients(state: &AppState) {
    for client in &state.clients {
        Dispatch::call(DispatchType::CloseWindow(WindowIdentifier::Address(
            client.address.clone(),
        )))
        .expect("Failed to close client");
    }
}
