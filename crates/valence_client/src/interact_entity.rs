use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use glam::Vec3;
use valence_core::hand::Hand;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{packet_id, Decode, Encode, Packet};
use valence_entity::EntityManager;

use crate::event_loop::{EventLoopPreUpdate, PacketEvent};

pub(super) fn build(app: &mut App) {
    app.add_event::<InteractEntityEvent>()
        .add_systems(EventLoopPreUpdate, handle_interact_entity);
}

#[derive(Event, Copy, Clone, Debug)]
pub struct InteractEntityEvent {
    pub client: Entity,
    /// The entity being interacted with.
    pub entity: Entity,
    /// If the client was sneaking during the interaction.
    pub sneaking: bool,
    /// The kind of interaction that occurred.
    pub interact: EntityInteraction,
}

#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode)]
pub enum EntityInteraction {
    Interact(Hand),
    Attack,
    InteractAt { target: Vec3, hand: Hand },
}

fn handle_interact_entity(
    mut packets: EventReader<PacketEvent>,
    entities: Res<EntityManager>,
    mut events: EventWriter<InteractEntityEvent>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<PlayerInteractEntityC2s>() {
            // TODO: check that the entity is in the same instance as the player.
            // TODO: check that the distance between the player and the interacted entity is
            // within some configurable tolerance level.

            if let Some(entity) = entities.get_by_id(pkt.entity_id.0) {
                events.send(InteractEntityEvent {
                    client: packet.client,
                    entity,
                    sneaking: pkt.sneaking,
                    interact: pkt.interact,
                })
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_INTERACT_ENTITY_C2S)]
pub struct PlayerInteractEntityC2s {
    pub entity_id: VarInt,
    pub interact: EntityInteraction,
    pub sneaking: bool,
}
