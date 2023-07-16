use std::time::Duration;

use bevy_app::App;
use valence::client::despawn_disconnected_clients;
use valence::client::message::ChatMessageEvent;
use valence::inventory::HeldItem;
use valence::prelude::*;
use valence::world_border::*;
use valence_client::message::SendMessage;

const SPAWN_Y: i32 = 64;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                init_clients,
                despawn_disconnected_clients,
                border_center_avg,
                border_expand,
                border_controls,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    biomes: Res<BiomeRegistry>,
    dimensions: Res<DimensionTypeRegistry>,
) {
    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            layer
                .chunk
                .set_block([x, SPAWN_Y, z], BlockState::MOSSY_COBBLESTONE);
        }
    }

    commands
        .spawn(layer)
        .insert(WorldBorderBundle::new([0.0, 0.0], 1.0));
}

fn init_clients(
    mut clients: Query<
        (
            &mut Client,
            &mut EntityLayerId,
            &mut Position,
            &mut Inventory,
            &HeldItem,
        ),
        Added<Client>,
    >,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut client, mut loc, mut pos, mut inv, main_slot) in &mut clients {
        loc.0 = instances.single();
        pos.set([0.5, SPAWN_Y as f64 + 1.0, 0.5]);
        let pickaxe = Some(ItemStack::new(ItemKind::WoodenPickaxe, 1, None));
        inv.set_slot(main_slot.slot(), pickaxe);
        client.send_chat_message("Break block to increase border size!");
    }
}

fn border_center_avg(
    clients: Query<(&EntityLayerId, &Position)>,
    mut instances: Query<(Entity, &mut WorldBorderCenter), With<Instance>>,
) {
    for (entity, mut center) in instances.iter_mut() {
        let new_center = {
            let (count, x, z) = clients
                .iter()
                .filter(|(loc, _)| loc.0 == entity)
                .fold((0, 0.0, 0.0), |(count, x, z), (_, pos)| {
                    (count + 1, x + pos.0.x, z + pos.0.z)
                });

            DVec2 {
                x: x / count.max(1) as f64,
                y: z / count.max(1) as f64,
            }
        };

        center.0 = new_center;
    }
}

fn border_expand(
    mut events: EventReader<DiggingEvent>,
    clients: Query<&EntityLayerId, With<Client>>,
    wbs: Query<&WorldBorderDiameter, With<Instance>>,
    mut event_writer: EventWriter<SetWorldBorderSizeEvent>,
) {
    for digging in events.iter().filter(|d| d.state == DiggingState::Stop) {
        let Ok(loc) = clients.get(digging.client) else {
            continue;
        };

        let Ok(size) = wbs.get(loc.0) else {
            continue;
        };

        event_writer.send(SetWorldBorderSizeEvent {
            entity_layer: loc.0,
            new_diameter: size.get() + 1.0,
            duration: Duration::from_secs(1),
        });
    }
}

// Not needed for this demo, but useful for debugging
fn border_controls(
    mut events: EventReader<ChatMessageEvent>,
    mut instances: Query<(Entity, &WorldBorderDiameter, &mut WorldBorderCenter), With<Instance>>,
    mut event_writer: EventWriter<SetWorldBorderSizeEvent>,
) {
    for x in events.iter() {
        let parts: Vec<&str> = x.message.split(' ').collect();
        match parts[0] {
            "add" => {
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
                    entity_layer: entity,
                    new_diameter: diameter.get() + value,
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
