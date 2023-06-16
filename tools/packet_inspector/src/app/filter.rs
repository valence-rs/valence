use egui::{RichText, Ui, Widget};
use itertools::Itertools;
use packet_inspector::PacketState;

use crate::tri_checkbox::{TriCheckbox, TriCheckboxState};

use super::{SharedState, Tab, View};

pub struct Filter {}

impl Tab for Filter {
    fn new() -> Self {
        Self {}
    }

    fn name(&self) -> &'static str {
        "Filters"
    }
}

impl View for Filter {
    fn ui(&mut self, ui: &mut egui::Ui, state: &mut SharedState) {
        draw_packet_list(ui, state, PacketState::Handshaking);
        ui.separator();
        draw_packet_list(ui, state, PacketState::Status);
        ui.separator();
        draw_packet_list(ui, state, PacketState::Login);
        ui.separator();
        draw_packet_list(ui, state, PacketState::Play);
    }
}

fn get_checkbox_state(state: &SharedState, packet_state: PacketState) -> TriCheckboxState {
    let mut p_enabled = 0;
    let mut disabled = 0;
    for (_, enabled) in state
        .packet_filter
        .iter()
        .filter(|(p, _)| p.state == packet_state)
    {
        if *enabled {
            p_enabled += 1;
        } else {
            disabled += 1;
        }
    }
    if p_enabled > 0 && disabled == 0 {
        TriCheckboxState::Enabled
    } else if p_enabled > 0 && disabled > 0 {
        TriCheckboxState::Partial
    } else {
        TriCheckboxState::Disabled
    }
}

fn draw_packet_list(ui: &mut Ui, state: &mut SharedState, packet_state: PacketState) {
    let title = match packet_state {
        PacketState::Handshaking => "Handshaking",
        PacketState::Status => "Status",
        PacketState::Login => "Login",
        PacketState::Play => "Play",
    };

    let mut checkbox = get_checkbox_state(state, packet_state);
    if TriCheckbox::new(&mut checkbox, RichText::new(title).heading().strong())
        .ui(ui)
        .changed()
    {
        for (_, enabled) in state
            .packet_filter
            .iter_mut()
            .filter(|(p, _)| p.state == packet_state)
        {
            if checkbox == TriCheckboxState::Partial {
                continue;
            }
            *enabled = checkbox == TriCheckboxState::Enabled;
        }
    }

    for (p, enabled) in state
        .packet_filter
        .iter_mut()
        .filter(|(p, _)| p.state == packet_state)
        .sorted_by(|(a, _), (b, _)| a.id.cmp(&b.id))
    {
        ui.checkbox(enabled, format!("[0x{:0>2X}] {}", p.id, p.name));
    }
}
