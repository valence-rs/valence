use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ShouldRun;
use uuid::Uuid;
use valence_new::client::event::default_event_handler;
use valence_new::client::{despawn_disconnected_clients, Client};
use valence_new::config::{Config, ConnectionMode};
use valence_new::dimension::DimensionId;
use valence_new::instance::{Chunk, Instance};
use valence_new::player_list::{
    add_new_clients_to_player_list, remove_disconnected_clients_from_player_list, Entry,
    PlayerList, PlayerListEntry,
};
use valence_new::server::Server;
use valence_protocol::block::BlockState;
use valence_protocol::text::{Color, TextFormat};
use valence_protocol::types::GameMode;

const SPAWN_Y: i32 = 64;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    valence_new::run_server(
        Config::default().with_connection_mode(ConnectionMode::Offline),
        SystemStage::parallel()
            .with_system(setup.with_run_criteria(ShouldRun::once))
            .with_system(init_clients)
            .with_system(update_player_list)
            .with_system(default_event_handler())
            .with_system(despawn_disconnected_clients)
            .with_system(add_new_clients_to_player_list)
            .with_system(remove_disconnected_clients_from_player_list),
        (),
    )
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

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block_state([x, SPAWN_Y, z], BlockState::GRAY_WOOL);
        }
    }

    world.spawn(instance);

    let mut player_list = world.resource_mut::<PlayerList>();

    player_list.insert(
        Uuid::from_u128(1),
        PlayerListEntry::new().with_display_name(Some("persistent entry with no ping")),
    );
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
    mut player_list: ResMut<PlayerList>,
) {
    let instance = instances.get_single().unwrap();

    for mut client in &mut clients {
        client.teleport([0.0, SPAWN_Y as f64 + 1.0, 0.0], 0.0, 0.0);
        client.set_instance(instance);
        client.set_game_mode(GameMode::Creative);

        client.send_message(
            "Please open your player list (tab key)."
                .italic()
                .color(Color::WHITE),
        );

        let entry = PlayerListEntry::new()
            .with_username(client.username())
            .with_properties(client.properties()) // For the player's skin and cape.
            .with_game_mode(client.game_mode())
            .with_ping(0) // Use negative values to indicate missing.
            .with_display_name(Some("ඞ".color(Color::new(255, 87, 66))));

        player_list.insert(client.uuid(), entry);
    }
}

fn update_player_list(mut player_list: ResMut<PlayerList>, server: Res<Server>) {
    let tick = server.current_tick();

    player_list.set_header("Current tick: ".into_text() + tick);
    player_list
        .set_footer("Current tick but in purple: ".into_text() + tick.color(Color::LIGHT_PURPLE));

    if tick % server.tick_rate() == 0 {
        match player_list.entry(Uuid::from_u128(2)) {
            Entry::Occupied(oe) => {
                oe.remove();
            }
            Entry::Vacant(ve) => {
                let entry = PlayerListEntry::new()
                    .with_display_name(Some("Hello!"))
                    .with_ping(300);

                ve.insert(entry);
            }
        }
    }
}
