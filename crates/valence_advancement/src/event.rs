use bevy_ecs::prelude::{EventReader, EventWriter};
use valence_client::event_loop::PacketEvent;
use valence_core::{packet::c2s::play::AdvancementTabC2s, ident::Ident};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AdvancementTabChange {
    OpenedTab { tab_id: Ident<String> },
    Closed,
}

pub(crate) fn handle_advancement_tab_change(
    mut packets: EventReader<PacketEvent>,
    mut advancement_tab_change_events: EventWriter<AdvancementTabChange>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<AdvancementTabC2s>() {
            advancement_tab_change_events.send(match pkt {
                AdvancementTabC2s::ClosedScreen => AdvancementTabChange::Closed,
                AdvancementTabC2s::OpenedTab { tab_id } => AdvancementTabChange::OpenedTab { 
                    tab_id: Ident::new_unchecked(tab_id.into_inner().to_string()) 
                }
            })
        }
    }
} 