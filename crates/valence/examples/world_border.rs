use std::time::Duration;

use valence::client::chat::ChatMessageEvent;
use valence::client::despawn_disconnected_clients;
use valence::client::world_border::{
    SetWorldBorderSizeEvent, WorldBorderBundle, WorldBorderCenter, WorldBorderDiameter,
};
use valence::prelude::*;

const SPAWN_Y: i32 = 64;

fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_system(border_controls)
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    biomes: Res<BiomeRegistry>,
    dimensions: Res<DimensionTypeRegistry>,
) {
    let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

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

    commands
        .spawn(instance)
        .insert(WorldBorderBundle::new([0.0, 0.0], 10.0));
}

fn init_clients(
    mut clients: Query<(&mut Client, &mut Location, &mut Position), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut client, mut loc, mut pos) in &mut clients {
        loc.0 = instances.single();
        pos.set([0.5, SPAWN_Y as f64 + 1.0, 0.5]);
        client.send_message("Border control: ");
        client.send_message("- Modify size: add <amount(can be negative)> <time(ms)>");
        client.send_message("- Change center: center <x> <z>");
    }
}

fn border_controls(
    mut events: EventReader<ChatMessageEvent>,
    mut instances: Query<(Entity, &WorldBorderDiameter, &mut WorldBorderCenter), With<Instance>>,
    mut event_writer: EventWriter<SetWorldBorderSizeEvent>,
) {
    for x in events.iter() {
        let parts: Vec<&str> = x.message.split(' ').collect();
        match parts[0] {
            "resize" => {
                let Ok(value) = parts[1].parse::<f64>() else {
                    return;
                };

                let Ok(speed) = parts[2].parse::<i64>() else {
                    return;
                };

                let Ok((entity, diameter, _)) = instances.get_single_mut() else {
                    return;
                };

                event_writer.send(SetWorldBorderSizeEvent {
                    instance: entity,
                    new_diameter: diameter.diameter() + value,
                    duration: Duration::from_millis(speed as u64),
                })
            }
            "center" => {
                let Ok(x) = parts[1].parse::<f64>() else {
                    return;
                };

                let Ok(z) = parts[2].parse::<f64>() else {
                    return;
                };

                instances.single_mut().2 .0 = DVec2 { x, y: z };
            }
            _ => (),
        }
    }
}
