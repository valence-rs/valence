#![allow(clippy::type_complexity)]

use valence::client::misc::Respawn;
use valence::prelude::*;

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()).with_connection_mode(ConnectionMode::Offline))
        .add_startup_system(setup)
        .add_systems((init_clients, squat_and_die, necromancy))
        .add_systems(PlayerList::default_systems())
        .add_system(despawn_disconnected_clients)
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Query<&DimensionType>,
    biomes: Query<&Biome>,
) {
    for block in [BlockState::GRASS_BLOCK, BlockState::DEEPSLATE] {
        let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

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
    mut clients: Query<
        (
            &mut Client,
            &mut Location,
            &mut Position,
            &mut HasRespawnScreen,
        ),
        Added<Client>,
    >,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut client, mut loc, mut pos, mut has_respawn_screen) in &mut clients {
        loc.0 = instances.iter().next().unwrap();
        pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        has_respawn_screen.0 = true;

        client.send_message(
            "Welcome to Valence! Sneak to die in the game (but not in real life).".italic(),
        );
    }
}

fn squat_and_die(mut clients: Query<&mut Client>, mut events: EventReader<Sneaking>) {
    for event in events.iter() {
        if event.state == SneakState::Start {
            if let Ok(mut client) = clients.get_mut(event.client) {
                client.kill(None, "Squatted too hard.");
            }
        }
    }
}

fn necromancy(
    mut clients: Query<(&mut Position, &mut Look, &mut Location)>,
    mut events: EventReader<Respawn>,
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
