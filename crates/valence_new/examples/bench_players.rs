use bevy_app::App;
use bevy_ecs::prelude::*;
use valence_new::client::event::default_event_handler;
use valence_new::client::{despawn_disconnected_clients, Client};
use valence_new::config::ServerPlugin;
use valence_new::dimension::DimensionId;
use valence_new::entity::{EntityKind, McEntity};
use valence_new::instance::{Chunk, Instance};
use valence_new::player_list::{
    add_new_clients_to_player_list, remove_disconnected_clients_from_player_list, PlayerList,
    PlayerListEntry,
};
use valence_new::protocol::block::BlockState;
use valence_new::protocol::types::GameMode;
use valence_new::server::Server;

const SPAWN_Y: i32 = 64;

fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(default_event_handler)
        .add_system(despawn_disconnected_clients)
        .add_system(add_new_clients_to_player_list)
        .add_system(remove_disconnected_clients_from_player_list)
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

    for z in -50..50 {
        for x in -50..50 {
            instance.set_block_state([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    world.spawn(instance);
}

fn init_clients(
    mut clients: Query<(Entity, &mut Client), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
    mut commands: Commands,
) {
    let instance = instances.get_single().unwrap();

    for (client_entity, mut client) in &mut clients {
        client.set_position([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        client.set_instance(instance);
        client.set_game_mode(GameMode::Creative);

        let player_entity = McEntity::with_uuid(EntityKind::Player, instance, client.uuid());

        commands.entity(client_entity).insert(player_entity);
    }
}
