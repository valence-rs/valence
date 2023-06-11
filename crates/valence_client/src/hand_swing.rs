use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_core::hand::Hand;
use valence_core::protocol::{packet_id, Decode, Encode, Packet};
use valence_entity::{EntityAnimation, EntityAnimations};

use crate::event_loop::{EventLoopSchedule, EventLoopSet, PacketEvent};

pub(super) fn build(app: &mut App) {
    app.add_event::<HandSwingEvent>().add_system(
        handle_hand_swing
            .in_schedule(EventLoopSchedule)
            .in_base_set(EventLoopSet::PreUpdate),
    );
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct HandSwingEvent {
    pub client: Entity,
    pub hand: Hand,
}

fn handle_hand_swing(
    mut packets: EventReader<PacketEvent>,
    mut clients: Query<&mut EntityAnimations>,
    mut events: EventWriter<HandSwingEvent>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<HandSwingC2s>() {
            if let Ok(mut anim) = clients.get_mut(packet.client) {
                anim.trigger(match pkt.hand {
                    Hand::Main => EntityAnimation::SwingMainHand,
                    Hand::Off => EntityAnimation::SwingOffHand,
                });
            }

            events.send(HandSwingEvent {
                client: packet.client,
                hand: pkt.hand,
            });
        }
    }
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::HAND_SWING_C2S)]
pub struct HandSwingC2s {
    pub hand: Hand,
}
