use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_entity::EntityManager;
pub use valence_protocol::packets::play::player_interact_entity_c2s::EntityInteraction;
use valence_protocol::packets::play::PlayerInteractEntityC2s;

use crate::event_loop::{EventLoopPreUpdate, PacketEvent};

pub struct InteractEntityPlugin;

impl Plugin for InteractEntityPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<InteractEntityEvent>()
            .add_systems(EventLoopPreUpdate, handle_interact_entity);
    }
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

fn handle_interact_entity(
    mut packets: EventReader<PacketEvent>,
    entities: Res<EntityManager>,
    mut events: EventWriter<InteractEntityEvent>,
) {
    for packet in packets.read() {
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
                });
            }
        }
    }
}
