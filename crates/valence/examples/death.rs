#![allow(clippy::type_complexity)]

use valence::client::event::{PerformRespawn, StartSneaking};
use valence::client::{default_event_handler, despawn_disconnected_clients};
use valence::entity::player::PlayerEntityBundle;
use valence::prelude::*;

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()).with_connection_mode(ConnectionMode::Offline))
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_systems(
            (default_event_handler, squat_and_die, necromancy).in_schedule(EventLoopSchedule),
        )
        .add_systems(PlayerList::default_systems())
        .add_system(despawn_disconnected_clients)
        .run();
}

fn setup(mut commands: Commands, server: Res<Server>) {
    for block in [BlockState::GRASS_BLOCK, BlockState::DEEPSLATE] {
        let mut instance = server.new_instance(DimensionId::default());

        for z in -5..5 {
            for x in -5..5 {
                instance.insert_chunk([x, z], Chunk::default());
            }
        }

        for z in -25..25 {
            for x in -25..25 {
                instance.set_block([x, SPAWN_Y, z], block);
            }
        }

        commands.spawn(instance);
    }
}

fn init_clients(
    mut clients: Query<(Entity, &UniqueId, &mut Client, &mut HasRespawnScreen), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
    mut commands: Commands,
) {
    for (entity, uuid, mut client, mut has_respawn_screen) in &mut clients {
        has_respawn_screen.0 = true;
        client.send_message(
            "Welcome to Valence! Sneak to die in the game (but not in real life).".italic(),
        );

        commands.entity(entity).insert(PlayerEntityBundle {
            location: Location(instances.iter().next().unwrap()),
            position: Position::new([0.0, SPAWN_Y as f64 + 1.0, 0.0]),
            uuid: *uuid,
            ..Default::default()
        });
    }
}

fn squat_and_die(mut clients: Query<&mut Client>, mut events: EventReader<StartSneaking>) {
    for event in events.iter() {
        if let Ok(mut client) = clients.get_mut(event.client) {
            client.kill(None, "Squatted too hard.");
        }
    }
}

fn necromancy(
    mut clients: Query<(&mut Position, &mut Look, &mut Location)>,
    mut events: EventReader<PerformRespawn>,
    instances: Query<Entity, With<Instance>>,
) {
    for event in events.iter() {
        if let Ok((mut pos, mut look, mut loc)) = clients.get_mut(event.client) {
            pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
            look.yaw = 0.0;
            look.pitch = 0.0;

            // make the client respawn in another instance
            let idx = instances.iter().position(|i| i == loc.0).unwrap();

            let count = instances.iter().count();

            loc.0 = instances.into_iter().nth((idx + 1) % count).unwrap();
        }
    }
}
