#![allow(clippy::type_complexity)]

use bevy_ecs::query::WorldQuery;
use glam::Vec3Swizzles;
use valence::client::event::{PlayerInteractEntity, StartSprinting, StopSprinting};
use valence::client::{default_event_handler, despawn_disconnected_clients};
use valence::entity::player::PlayerEntityBundle;
use valence::entity::EntityStatuses;
use valence::prelude::*;

const SPAWN_Y: i32 = 64;
const ARENA_RADIUS: i32 = 32;

/// Attached to every client.
#[derive(Component)]
struct CombatState {
    /// The tick the client was last attacked.
    last_attacked_tick: i64,
    has_bonus_knockback: bool,
}

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_systems((default_event_handler, handle_combat_events).in_schedule(EventLoopSchedule))
        .add_systems(PlayerList::default_systems())
        .add_system(despawn_disconnected_clients)
        .add_system(teleport_oob_clients)
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Query<&DimensionType>,
    biomes: Query<&Biome>,
) {
    let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    // Create circular arena.
    for z in -ARENA_RADIUS..ARENA_RADIUS {
        for x in -ARENA_RADIUS..ARENA_RADIUS {
            let dist = f64::hypot(x as _, z as _) / ARENA_RADIUS as f64;

            if dist > 1.0 {
                continue;
            }

            let block = if rand::random::<f64>() < dist {
                BlockState::STONE
            } else {
                BlockState::DEEPSLATE
            };

            for y in 0..SPAWN_Y {
                instance.set_block([x, y, z], block);
            }
        }
    }

    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<(Entity, &UniqueId), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
    mut commands: Commands,
) {
    for (entity, uuid) in &mut clients {
        commands.entity(entity).insert((
            CombatState {
                last_attacked_tick: 0,
                has_bonus_knockback: false,
            },
            PlayerEntityBundle {
                location: Location(instances.single()),
                position: Position::new([0.5, SPAWN_Y as f64, 0.5]),
                uuid: *uuid,
                ..Default::default()
            },
        ));
    }
}

#[derive(WorldQuery)]
#[world_query(mutable)]
struct CombatQuery {
    client: &'static mut Client,
    pos: &'static Position,
    state: &'static mut CombatState,
    statuses: &'static mut EntityStatuses,
}

fn handle_combat_events(
    manager: Res<EntityManager>,
    server: Res<Server>,
    mut clients: Query<CombatQuery>,
    mut start_sprinting: EventReader<StartSprinting>,
    mut stop_sprinting: EventReader<StopSprinting>,
    mut interact_with_entity: EventReader<PlayerInteractEntity>,
) {
    for &StartSprinting { client } in start_sprinting.iter() {
        if let Ok(mut client) = clients.get_mut(client) {
            client.state.has_bonus_knockback = true;
        }
    }

    for &StopSprinting { client } in stop_sprinting.iter() {
        if let Ok(mut client) = clients.get_mut(client) {
            client.state.has_bonus_knockback = false;
        }
    }

    for &PlayerInteractEntity {
        client: attacker_client,
        entity_id,
        ..
    } in interact_with_entity.iter()
    {
        let Some(victim_client) = manager.get_with_id(entity_id) else {
            // Attacked entity doesn't exist.
            continue
        };

        let Ok([mut attacker, mut victim]) = clients.get_many_mut([attacker_client, victim_client]) else {
            // Victim or attacker does not exist, or the attacker is attacking itself.
            continue
        };

        if server.current_tick() - victim.state.last_attacked_tick < 10 {
            // Victim is still on attack cooldown.
            continue;
        }

        victim.state.last_attacked_tick = server.current_tick();

        let victim_pos = victim.pos.0.xz();
        let attacker_pos = attacker.pos.0.xz();

        let dir = (victim_pos - attacker_pos).normalize().as_vec2();

        let knockback_xz = if attacker.state.has_bonus_knockback {
            18.0
        } else {
            8.0
        };
        let knockback_y = if attacker.state.has_bonus_knockback {
            8.432
        } else {
            6.432
        };

        victim
            .client
            .set_velocity([dir.x * knockback_xz, knockback_y, dir.y * knockback_xz]);

        attacker.state.has_bonus_knockback = false;

        victim.client.trigger_status(EntityStatus::PlayAttackSound);

        victim.statuses.trigger(EntityStatus::PlayAttackSound);
    }
}

fn teleport_oob_clients(mut clients: Query<&mut Position, With<Client>>) {
    for mut pos in &mut clients {
        if pos.0.y < 0.0 {
            pos.set([0.0, SPAWN_Y as _, 0.0]);
        }
    }
}
