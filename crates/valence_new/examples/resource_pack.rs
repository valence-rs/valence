use valence_new::client::despawn_disconnected_clients;
use valence_new::client::event::{
    default_event_handler, InteractWithEntity, ResourcePackStatus, ResourcePackStatusChange,
};
use valence_new::prelude::*;
use valence_protocol::types::EntityInteraction;

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_system_to_stage(EventLoop, prompt_on_punch)
        .add_system_to_stage(EventLoop, on_resource_pack_status)
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
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

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block_state([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    let instance_ent = world.spawn(instance).id();

    let mut sheep = McEntity::new(EntityKind::Sheep, instance_ent);
    sheep.set_position([0.0, SPAWN_Y as f64 + 1.0, 2.0]);
    world.spawn(sheep);
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    let instance = instances.get_single().unwrap();

    for mut client in &mut clients {
        client.set_position([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        client.set_instance(instance);
        client.set_game_mode(GameMode::Creative);

        client.send_message("Hit the sheep to prompt for the resource pack.".italic());
    }
}

fn prompt_on_punch(mut clients: Query<&mut Client>, mut events: EventReader<InteractWithEntity>) {
    for event in events.iter() {
        let Ok(mut client) = clients.get_mut(event.client) else {
            continue;
        };
        if event.interact == EntityInteraction::Attack {
            client.set_resource_pack(
                "https://github.com/valence-rs/valence/raw/main/assets/example_pack.zip",
                "d7c6108849fb190ec2a49f2d38b7f1f897d9ce9f",
                false,
                Option::<Text>::None,
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
