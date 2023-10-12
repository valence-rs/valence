use rand::seq::SliceRandom;
use rand::Rng;
use valence::client::despawn_disconnected_clients;
use valence::entity::active_status_effects::{ActiveStatusEffect, ActiveStatusEffects};
use valence::log::LogPlugin;
use valence::network::ConnectionMode;
use valence::prelude::*;
use valence::status_effects::StatusEffect;

const SPAWN_Y: i32 = 64;

fn main() {
    App::new()
        .insert_resource(NetworkSettings {
            connection_mode: ConnectionMode::Offline,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins.build().disable::<LogPlugin>())
        .add_systems(Startup, setup)
        .add_systems(EventLoopUpdate, add_potion_effect)
        .add_systems(Update, (init_clients, despawn_disconnected_clients))
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
                .set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(layer);
}

#[allow(clippy::type_complexity)]
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
        *game_mode = GameMode::Survival;

        client.send_chat_message("Welcome to the potions example.".bold());
        client.send_chat_message("Sneak to apply a random potion effect.".into_text());
        client.send_chat_message("Note: Most potion effects are not implemented yet.".into_text());
    }
}

pub fn add_potion_effect(
    mut clients: Query<&mut ActiveStatusEffects>,
    mut events: EventReader<SneakEvent>,
) {
    let mut rng = rand::thread_rng();
    for event in events.iter() {
        if event.state == SneakState::Start {
            if let Ok(mut status) = clients.get_mut(event.client) {
                status.apply(
                    ActiveStatusEffect::from_effect(*StatusEffect::ALL.choose(&mut rng).unwrap())
                        .with_duration(rng.gen_range(10..1000))
                        .with_amplifier(rng.gen_range(0..5)),
                );
            }
        }
    }
}
