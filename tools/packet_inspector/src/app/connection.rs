use crate::shared_state::Event;

use super::{SharedState, Tab, View};

pub struct Connection {}

impl Tab for Connection {
    fn new() -> Self {
        Self {}
    }

    fn name(&self) -> &'static str {
        "Connection"
    }
}

impl View for Connection {
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut SharedState) {
        if state.is_listening {
            ui.label("Listener Address");
            ui.text_edit_singleline(&mut state.listener_addr.clone());
            ui.label("Server Address");
            ui.text_edit_singleline(&mut state.server_addr.clone());
            if ui.button("Stop Listening").clicked() {
                state.send_event(Event::StopListening);
            }
        } else {
            ui.label("Listener Address");
            ui.text_edit_singleline(&mut state.listener_addr);
            ui.label("Server Address");
            ui.text_edit_singleline(&mut state.server_addr);
            if ui.button("Start Listening").clicked() {
                state.send_event(Event::StartListening);
            }
        }
    }
}
