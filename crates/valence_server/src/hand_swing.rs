use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_entity::{EntityAnimation, EntityAnimations};
use valence_protocol::packets::play::HandSwingC2s;
use valence_protocol::Hand;

use crate::event_loop::{EventLoopPreUpdate, PacketEvent};

pub struct HandSwingPlugin;

impl Plugin for HandSwingPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<HandSwingEvent>()
            .add_systems(EventLoopPreUpdate, handle_hand_swing);
    }
}

#[derive(Event, Copy, Clone, PartialEq, Eq, Debug)]
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
