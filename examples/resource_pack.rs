#![allow(clippy::type_complexity)]

use valence::entity::sheep::SheepEntityBundle;
use valence::message::SendMessage;
use valence::prelude::*;
use valence::protocol::packets::play::ResourcePackStatusC2s;
use valence::resource_pack::ResourcePackStatusEvent;

const SPAWN_Y: i32 = 64;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                init_clients,
                prompt_on_punch,
                on_resource_pack_status,
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
    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            layer.chunk.insert_chunk([x, z], Chunk::new());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            layer.chunk.set_block([x, SPAWN_Y, z], BlockState::BEDROCK);
        }
    }

    let layer_ent = commands.spawn(layer).id();

    commands.spawn(SheepEntityBundle {
        layer: EntityLayerId(layer_ent),
        position: Position::new([0.0, SPAWN_Y as f64 + 1.0, 2.0]),
        look: Look::new(180.0, 0.0),
        head_yaw: HeadYaw(180.0),
        ..Default::default()
    });
}

fn init_clients(
    mut clients: Query<
        (
            &mut Client,
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for (
        mut client,
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut game_mode,
    ) in &mut clients
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        *game_mode = GameMode::Creative;

        client.send_chat_message("Hit the sheep to prompt for the resource pack.".italic());
    }
}

fn prompt_on_punch(mut clients: Query<&mut Client>, mut events: EventReader<InteractEntityEvent>) {
    for event in events.iter() {
        if let Ok(mut client) = clients.get_mut(event.client) {
            if event.interact == EntityInteraction::Attack {
                client.set_resource_pack(
                    "https://github.com/valence-rs/valence/raw/main/assets/example_pack.zip",
                    "d7c6108849fb190ec2a49f2d38b7f1f897d9ce9f",
                    false,
                    None,
                );
            }
        };
    }
}

fn on_resource_pack_status(
    mut clients: Query<&mut Client>,
    mut events: EventReader<ResourcePackStatusEvent>,
) {
    for event in events.iter() {
        if let Ok(mut client) = clients.get_mut(event.client) {
            match event.status {
                ResourcePackStatusC2s::Accepted => {
                    client.send_chat_message("Resource pack accepted.".color(Color::GREEN));
                }
                ResourcePackStatusC2s::Declined => {
                    client.send_chat_message("Resource pack declined.".color(Color::RED));
                }
                ResourcePackStatusC2s::FailedDownload => {
                    client.send_chat_message("Resource pack failed to download.".color(Color::RED));
                }
                ResourcePackStatusC2s::SuccessfullyLoaded => {
                    client.send_chat_message(
                        "Resource pack successfully downloaded.".color(Color::BLUE),
                    );
                }
            }
        };
    }
}
