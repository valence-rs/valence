use glam::Vec3Swizzles;
use valence_new::client::despawn_disconnected_clients;
use valence_new::client::event::{
    default_event_handler, InteractWithEntity, StartSprinting, StopSprinting,
};
use valence_new::player_list::{
    add_new_clients_to_player_list, remove_disconnected_clients_from_player_list,
};
use valence_new::prelude::*;

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
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_system_to_stage(EventLoop, handle_combat_events)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_system(add_new_clients_to_player_list)
        .add_system(remove_disconnected_clients_from_player_list)
        .add_system(teleport_oob_clients)
        .run();
}

fn setup(world: &mut World) {
    let mut instance = world
        .resource::<Server>()
        .new_instance(DimensionId::default());

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
                instance.set_block_state([x, y, z], block);
            }
        }
    }

    world.spawn(instance);
}

fn init_clients(
    mut commands: Commands,
    mut clients: Query<(Entity, &mut Client), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    let instance = instances.get_single().unwrap();

    for (entity, mut client) in &mut clients {
        client.set_position([0.0, SPAWN_Y as f64, 0.0]);
        client.set_instance(instance);

        commands.entity(entity).insert((
            CombatState {
                last_attacked_tick: 0,
                has_bonus_knockback: false,
            },
            McEntity::with_uuid(EntityKind::Player, instance, client.uuid()),
        ));
    }
}

fn handle_combat_events(
    manager: Res<McEntityManager>,
    server: Res<Server>,
    mut start_sprinting: EventReader<StartSprinting>,
    mut stop_sprinting: EventReader<StopSprinting>,
    mut interact_with_entity: EventReader<InteractWithEntity>,
    mut clients: Query<(&mut Client, &mut CombatState)>,
) {
    for &StartSprinting { client } in start_sprinting.iter() {
        if let Ok((_, mut state)) = clients.get_mut(client) {
            state.has_bonus_knockback = true;
        }
    }

    for &StopSprinting { client } in stop_sprinting.iter() {
        if let Ok((_, mut state)) = clients.get_mut(client) {
            state.has_bonus_knockback = false;
        }
    }

    for &InteractWithEntity {
        client: attacker_client,
        entity_id,
        ..
    } in interact_with_entity.iter()
    {
        let Some(victim_client) = manager.get_with_protocol_id(entity_id) else {
            // Attacked entity doesn't exist.
            continue
        };

        let Ok([(attacker_client, mut attacker_state), (mut victim_client, mut victim_state)]) =
            clients.get_many_mut([attacker_client, victim_client])
        else {
            // Victim or attacker does not exist, or the attacker is attacking itself.
            continue
        };

        if server.current_tick() - victim_state.last_attacked_tick < 10 {
            // Victim is still on attack cooldown.
            continue;
        }

        victim_state.last_attacked_tick = server.current_tick();

        let victim_pos = victim_client.position().xz();
        let attacker_pos = attacker_client.position().xz();

        let dir = (victim_pos - attacker_pos).normalize().as_vec2();

        let knockback_xz = if attacker_state.has_bonus_knockback {
            18.0
        } else {
            8.0
        };
        let knockback_y = if attacker_state.has_bonus_knockback {
            8.432
        } else {
            6.432
        };

        victim_client.set_velocity([dir.x * knockback_xz, knockback_y, dir.y * knockback_xz]);

        attacker_state.has_bonus_knockback = false;

        // TODO: trigger statuses.
    }
}

fn teleport_oob_clients(mut clients: Query<&mut Client>) {
    for mut client in &mut clients {
        if client.position().y < 0.0 {
            client.set_position([0.0, SPAWN_Y as _, 0.0]);
        }
    }
}
