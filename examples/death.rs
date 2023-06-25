#![allow(clippy::type_complexity)]

use valence::prelude::*;
use valence_client::message::SendMessage;
use valence_client::status::RequestRespawnEvent;

const SPAWN_Y: i32 = 64;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                init_clients,
                squat_and_die,
                necromancy,
                despawn_disconnected_clients,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    for block in [
        BlockState::GRASS_BLOCK,
        BlockState::DEEPSLATE,
        BlockState::MAGMA_BLOCK,
    ] {
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
    mut clients: Query<(&mut Client, &mut Location, &mut Position), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut client, mut loc, mut pos) in &mut clients {
        loc.0 = instances.iter().next().unwrap();
        pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);

        client.send_chat_message(
            "Welcome to Valence! Sneak to die in the game (but not in real life).".italic(),
        );
    }
}

fn squat_and_die(mut clients: Query<&mut Client>, mut events: EventReader<SneakEvent>) {
    for event in events.iter() {
        if event.state == SneakState::Start {
            if let Ok(mut client) = clients.get_mut(event.client) {
                client.kill("Squatted too hard.");
            }
        }
    }
}

fn necromancy(
    mut clients: Query<(&mut Position, &mut Look, &mut Location)>,
    mut events: EventReader<RequestRespawnEvent>,
    instances: Query<Entity, With<Instance>>,
) {
    for event in events.iter() {
        if let Ok((mut loc, mut spawn_pos)) = clients.get_mut(event.client) {
            spawn_pos.pos = BlockPos::new(0, SPAWN_Y, 0);

            // make the client respawn in another instance
            let idx = instances.iter().position(|i| i == loc.0).unwrap();

            let count = instances.iter().len();

            loc.0 = instances.into_iter().nth((idx + 1) % count).unwrap();
        }
    }
}
