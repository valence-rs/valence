use tracing::warn;
use valence_new::client::despawn_disconnected_clients;
use valence_new::client::event::{default_event_handler, PerformRespawn, StartSneaking};
use valence_new::prelude::*;

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_system_to_stage(EventLoop, squat_and_die)
        .add_system_to_stage(EventLoop, necromancy)
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .run();
}

fn setup(world: &mut World) {
    for block in [BlockState::GRASS_BLOCK, BlockState::DEEPSLATE] {
        let mut instance = world
            .resource::<Server>()
            .new_instance(DimensionId::default());

        for z in -5..5 {
            for x in -5..5 {
                instance.insert_chunk([x, z], Chunk::default());
            }
        }

        for z in -25..25 {
            for x in -25..25 {
                instance.set_block_state([x, SPAWN_Y, z], block);
            }
        }

        world.spawn(instance);
    }
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    let instance = instances.into_iter().next().unwrap();

    for mut client in &mut clients {
        client.set_position([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        client.set_respawn_screen(true);
        client.set_instance(instance);
        client.send_message(
            "Welcome to Valence! Press shift to die in the game (but not in real life).".italic(),
        );
    }
}

fn squat_and_die(mut clients: Query<&mut Client>, mut events: EventReader<StartSneaking>) {
    for event in events.iter() {
        let Ok(mut client) = clients.get_component_mut::<Client>(event.client) else {
            warn!("Client {:?} not found", event.client);
            continue;
        };

        client.kill(None, "Squatted too hard.");
    }
}

fn necromancy(
    mut clients: Query<&mut Client>,
    mut events: EventReader<PerformRespawn>,
    instances: Query<Entity, With<Instance>>,
) {
    for event in events.iter() {
        let Ok(mut client) = clients.get_component_mut::<Client>(event.client) else {
            continue;
        };
        client.set_position([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        client.set_velocity([0.0, 0.0, 0.0]);
        client.set_yaw(0.0);
        client.set_pitch(0.0);
        // make the client respawn in another instance
        let idx = instances
            .iter()
            .position(|i| i == client.instance())
            .unwrap();
        let count = instances.iter().count();
        client.set_instance(instances.into_iter().nth((idx + 1) % count).unwrap());
    }
}
