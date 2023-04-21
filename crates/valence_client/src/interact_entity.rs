use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_core::packet::c2s::play::player_interact_entity::EntityInteraction;
use valence_core::packet::c2s::play::PlayerInteractEntityC2s;
use valence_entity::EntityManager;

use crate::event_loop::{EventLoopSchedule, EventLoopSet, PacketEvent};

pub(super) fn build(app: &mut App) {
    app.add_event::<InteractEntity>().add_system(
        handle_interact_entity
            .in_schedule(EventLoopSchedule)
            .in_base_set(EventLoopSet::PreUpdate),
    );
}

#[derive(Copy, Clone, Debug)]
pub struct InteractEntity {
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
    mut events: EventWriter<InteractEntity>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<PlayerInteractEntityC2s>() {
            // TODO: check that the entity is in the same instance as the player.
            // TODO: check that the distance between the player and the interacted entity is
            // within some configurable tolerance level.

            if let Some(entity) = entities.get_by_id(pkt.entity_id.0) {
                events.send(InteractEntity {
                    client: packet.client,
                    entity,
                    sneaking: pkt.sneaking,
                    interact: pkt.interact,
                })
            }
        }
    }
}
