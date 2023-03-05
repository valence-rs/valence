use valence::client::despawn_disconnected_clients;
use valence::client::event::default_event_handler;
use valence::prelude::*;

#[allow(unused_imports)]
use crate::extras::*;

const SPAWN_Y: i32 = 64;

pub fn build_app(app: &mut App) {
    app.add_plugin(ServerPlugin::new(()))
        .add_system(default_event_handler.in_schedule(EventLoopSchedule))
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients);
}

fn setup(mut commands: Commands, server: Res<Server>) {
    let mut instance = server.new_instance(DimensionId::default());

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    let instance = instances.get_single().unwrap();

    for mut client in &mut clients {
        client.set_position([0.5, SPAWN_Y as f64 + 1.0, 0.5]);
        client.set_instance(instance);
        client.set_game_mode(GameMode::Survival);
    }
}

// Add new systems here!
