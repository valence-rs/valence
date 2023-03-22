#![allow(clippy::type_complexity)]

use valence::client::event::{PlayerInteract, ResourcePackStatus, ResourcePackStatusChange};
use valence::client::{default_event_handler, despawn_disconnected_clients};
use valence::entity::player::PlayerBundle;
use valence::entity::sheep::SheepBundle;
use valence::prelude::*;
use valence::protocol::packet::c2s::play::player_interact::Interaction;

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_systems(
            (
                default_event_handler,
                prompt_on_punch,
                on_resource_pack_status,
            )
                .in_schedule(EventLoopSchedule),
        )
        .add_systems(PlayerList::default_systems())
        .add_system(despawn_disconnected_clients)
        .run();
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
            instance.set_block([x, SPAWN_Y, z], BlockState::BEDROCK);
        }
    }

    let instance_ent = commands.spawn(instance).id();

    commands.spawn(SheepBundle {
        location: Location(instance_ent),
        position: Position::new([0.0, SPAWN_Y as f64 + 1.0, 2.0]),
        look: Look::new(180.0, 0.0),
        head_yaw: HeadYaw(180.0),
        ..Default::default()
    });
}

fn init_clients(
    mut clients: Query<(Entity, &UniqueId, &mut Client, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
    mut commands: Commands,
) {
    for (entity, uuid, mut client, mut game_mode) in &mut clients {
        *game_mode = GameMode::Creative;

        client.send_message("Hit the sheep to prompt for the resource pack.".italic());

        commands.entity(entity).insert(PlayerBundle {
            location: Location(instances.single()),
            position: Position::new([0.0, SPAWN_Y as f64 + 1.0, 0.0]),
            uuid: *uuid,
            ..Default::default()
        });
    }
}

fn prompt_on_punch(mut clients: Query<&mut Client>, mut events: EventReader<PlayerInteract>) {
    for event in events.iter() {
        let Ok(mut client) = clients.get_mut(event.client) else {
            continue;
        };
        if event.interact == Interaction::Attack {
            client.set_resource_pack(
                "https://github.com/valence-rs/valence/raw/main/assets/example_pack.zip",
                "d7c6108849fb190ec2a49f2d38b7f1f897d9ce9f",
                false,
                None,
            );
        }
    }
}

fn on_resource_pack_status(
    mut clients: Query<&mut Client>,
    mut events: EventReader<ResourcePackStatusChange>,
) {
    for event in events.iter() {
        let Ok(mut client) = clients.get_mut(event.client) else {
            continue;
        };
        match event.status {
            ResourcePackStatus::Accepted => {
                client.send_message("Resource pack accepted.".color(Color::GREEN));
            }
            ResourcePackStatus::Declined => {
                client.send_message("Resource pack declined.".color(Color::RED));
            }
            ResourcePackStatus::FailedDownload => {
                client.send_message("Resource pack failed to download.".color(Color::RED));
            }
            ResourcePackStatus::Loaded => {
                client.send_message("Resource pack successfully downloaded.".color(Color::BLUE));
            }
        }
    }
}
