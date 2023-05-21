use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_core::hand::Hand;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{packet_id, Decode, Encode, Packet};

use crate::action::ActionSequence;
use crate::event_loop::{EventLoopSchedule, EventLoopSet, PacketEvent};

pub(super) fn build(app: &mut App) {
    app.add_event::<InteractItemEvent>().add_system(
        handle_player_interact_item
            .in_schedule(EventLoopSchedule)
            .in_base_set(EventLoopSet::PreUpdate),
    );
}

#[derive(Copy, Clone, Debug)]
pub struct InteractItemEvent {
    pub client: Entity,
    pub hand: Hand,
    pub sequence: i32,
}

fn handle_player_interact_item(
    mut packets: EventReader<PacketEvent>,
    mut clients: Query<&mut ActionSequence>,
    mut events: EventWriter<InteractItemEvent>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<PlayerInteractItemC2s>() {
            if let Ok(mut action_seq) = clients.get_mut(packet.client) {
                action_seq.update(pkt.sequence.0);
            }

            events.send(InteractItemEvent {
                client: packet.client,
                hand: pkt.hand,
                sequence: pkt.sequence.0,
            });
        }
    }
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_INTERACT_ITEM_C2S)]
pub struct PlayerInteractItemC2s {
    pub hand: Hand,
    pub sequence: VarInt,
}
