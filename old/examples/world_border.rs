#![allow(clippy::type_complexity)]

use bevy_app::App;
use valence::client::despawn_disconnected_clients;
use valence::inventory::HeldItem;
use valence::message::{ChatMessageEvent, SendMessage};
use valence::prelude::*;
use valence::world_border::*;

const SPAWN_Y: i32 = 64;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                despawn_disconnected_clients,
                init_clients,
                border_controls,
                display_diameter,
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

    commands.spawn((
        layer,
        WorldBorderBundle {
            lerp: WorldBorderLerp {
                target_diameter: 10.0,
                ..Default::default()
            },
            ..Default::default()
        },
    ));
}

fn init_clients(
    mut clients: Query<
        (
            &mut Client,
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut Inventory,
            &HeldItem,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, With<ChunkLayer>>,
) {
    for (
        mut client,
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut inv,
        main_slot,
    ) in &mut clients
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set([0.5, SPAWN_Y as f64 + 1.0, 0.5]);
        let pickaxe = ItemStack::new(ItemKind::WoodenPickaxe, 1, None);
        inv.set_slot(main_slot.slot(), pickaxe);
        client
            .send_chat_message("Use `add` and `center` chat messages to change the world border.");
    }
}

fn display_diameter(mut layers: Query<(&mut ChunkLayer, &WorldBorderLerp)>) {
    for (mut layer, lerp) in &mut layers {
        if lerp.remaining_ticks > 0 {
            layer.send_chat_message(format!("diameter = {}", lerp.current_diameter));
        }
    }
}

fn border_controls(
    mut events: EventReader<ChatMessageEvent>,
    mut layers: Query<(&mut WorldBorderCenter, &mut WorldBorderLerp), With<ChunkLayer>>,
) {
    for x in events.read() {
        let parts: Vec<&str> = x.message.split(' ').collect();
        match parts[0] {
            "add" => {
                let Ok(value) = parts[1].parse::<f64>() else {
                    return;
                };

                let Ok(ticks) = parts[2].parse::<u64>() else {
                    return;
                };

                let (_, mut lerp) = layers.single_mut();

                lerp.target_diameter = lerp.current_diameter + value;
                lerp.remaining_ticks = ticks;
            }
            "center" => {
                let Ok(x) = parts[1].parse::<f64>() else {
                    return;
                };

                let Ok(z) = parts[2].parse::<f64>() else {
                    return;
                };

                let (mut center, _) = layers.single_mut();
                center.x = x;
                center.z = z;
            }
            _ => (),
        }
    }
}
