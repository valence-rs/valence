use bevy_ecs::prelude::{Entity, EventReader, EventWriter};
use valence_client::event_loop::PacketEvent;
use valence_core::ident::Ident;
use valence_core::packet::c2s::play::AdvancementTabC2s;

/// This event sends when the client changes or closes advancement's tab.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AdvancementTabChange {
    pub client: Entity,
    /// If None then the client has closed advancement's tabs.
    pub opened_tab: Option<Ident<String>>,
}

pub(crate) fn handle_advancement_tab_change(
    mut packets: EventReader<PacketEvent>,
    mut advancement_tab_change_events: EventWriter<AdvancementTabChange>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<AdvancementTabC2s>() {
            advancement_tab_change_events.send(AdvancementTabChange {
                client: packet.client,
                opened_tab: match pkt {
                    AdvancementTabC2s::ClosedScreen => None,
                    AdvancementTabC2s::OpenedTab { tab_id } => {
                        Some(tab_id.into())
                    }
                },
            })
        }
    }
}
