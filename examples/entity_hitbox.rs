#![allow(clippy::type_complexity)]

use std::collections::HashMap;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use entity::NameVisible;
use valence::entity::hoglin::HoglinEntityBundle;
use valence::entity::pig::PigEntityBundle;
use valence::entity::sheep::SheepEntityBundle;
use valence::entity::warden::WardenEntityBundle;
use valence::entity::zombie::ZombieEntityBundle;
use valence::entity::zombie_horse::ZombieHorseEntityBundle;
use valence::entity::{entity, Pose};
use valence::prelude::*;
use valence::rand::Rng;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (init_clients, spawn_entity, intersections))
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
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            layer.chunk.set_block([x, 64, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(layer);
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
        pos.set([0.0, 65.0, 0.0]);
        *game_mode = GameMode::Creative;

        client.send_game_message("To spawn an entity, press shift. F3 + B to activate hitboxes");
    }
}

fn spawn_entity(
    mut commands: Commands,
    mut sneaking: EventReader<SneakEvent>,
    client_query: Query<(&Position, &EntityLayerId)>,
) {
    for sneaking in sneaking.iter() {
        if sneaking.state == SneakState::Start {
            continue;
        }

        let (position, layer) = client_query.get(sneaking.client).unwrap();

        let position = *position;
        let layer = *layer;

        match rand::thread_rng().gen_range(0..7) {
            0 => commands.spawn(SheepEntityBundle {
                position,
                layer,
                entity_name_visible: NameVisible(true),
                ..Default::default()
            }),
            1 => commands.spawn(PigEntityBundle {
                position,
                layer,
                entity_name_visible: NameVisible(true),
                ..Default::default()
            }),
            2 => commands.spawn(ZombieEntityBundle {
                position,
                layer,
                entity_name_visible: NameVisible(true),
                ..Default::default()
            }),
            3 => commands.spawn(ZombieHorseEntityBundle {
                position,
                layer,
                entity_name_visible: NameVisible(true),
                ..Default::default()
            }),
            4 => commands.spawn(WardenEntityBundle {
                position,
                layer,
                entity_name_visible: NameVisible(true),
                entity_pose: entity::Pose(Pose::Digging),
                ..Default::default()
            }),
            5 => commands.spawn(WardenEntityBundle {
                position,
                layer,
                entity_name_visible: NameVisible(true),
                ..Default::default()
            }),
            6 => commands.spawn(HoglinEntityBundle {
                position,
                layer,
                entity_name_visible: NameVisible(true),
                ..Default::default()
            }),
            _ => unreachable!(),
        };
    }
}

fn intersections(query: Query<(Entity, &Hitbox)>, mut name_query: Query<&mut entity::CustomName>) {
    // This code only to show how hitboxes can be used
    let mut intersections = HashMap::new();

    for [(entity1, hitbox1), (entity2, hitbox2)] in query.iter_combinations() {
        let aabb1 = hitbox1.get();
        let aabb2 = hitbox2.get();

        let _ = *intersections.entry(entity1).or_insert(0);
        let _ = *intersections.entry(entity2).or_insert(0);

        if aabb1.intersects(aabb2) {
            *intersections.get_mut(&entity1).unwrap() += 1;
            *intersections.get_mut(&entity2).unwrap() += 1;
        }
    }

    for (entity, value) in intersections {
        let Ok(mut name) = name_query.get_mut(entity) else {
            continue;
        };
        name.0 = Some(format!("{value}").into());
    }
}
