use valence_inventory::PlayerAction;
use valence_server::{entity::living::LivingFlags, event_loop::PacketEvent, interact_item::InteractItemEvent, protocol::packets::play::PlayerActionC2s};

use super::*;

/// This component will broadcast item interactions (e.g. drawing a bow, eating food) to other players by setting the "using_item" LivingFlag.
#[derive(Debug, Default, Clone, Component)]
pub struct EquipmentInteractionBroadcast;

// Sets "using_item" flag to true when the client starts interacting with an item.
pub(crate) fn start_interaction(
    mut clients: Query<&mut LivingFlags, (With<Client>, With<EquipmentInteractionBroadcast>)>,
    mut events: EventReader<InteractItemEvent>,
) {
    for event in events.read() {
        if let Ok(mut flags) = clients.get_mut(event.client) {
            flags.set_using_item(true);
        }
    }
}


// Sets "using_item" flag to false when the client stops interacting with an item.
pub(crate) fn stop_interaction(
    mut clients: Query<&mut LivingFlags, (With<Client>, With<EquipmentInteractionBroadcast>)>,
    mut packets: EventReader<PacketEvent>,
) {
    for packet in packets.read() {
        if let Some(pkt) = packet.decode::<PlayerActionC2s>() {
            if pkt.action == PlayerAction::ReleaseUseItem {
                if let Ok(mut flags) = clients.get_mut(packet.client) {
                    flags.set_using_item(false);
                }
            }
        }
    }
}