use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_protocol::packets::play::UseItemC2s;
use valence_protocol::Hand;

use crate::action::ActionSequence;
use crate::event_loop::{EventLoopPreUpdate, PacketEvent};

pub struct InteractItemPlugin;

impl Plugin for InteractItemPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<InteractItemEvent>()
            .add_systems(EventLoopPreUpdate, handle_player_interact_item);
    }
}

#[derive(Event, Copy, Clone, Debug)]
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
    for packet in packets.read() {
        if let Some(pkt) = packet.decode::<UseItemC2s>() {
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
