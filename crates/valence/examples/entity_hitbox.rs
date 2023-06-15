use std::collections::HashMap;

use bevy_app::App;
use bevy_ecs::prelude::Entity;
use rand::Rng;
use valence::prelude::*;
use valence_client::message::SendMessage;
use valence_entity::entity::NameVisible;
use valence_entity::hoglin::HoglinEntityBundle;
use valence_entity::pig::PigEntityBundle;
use valence_entity::sheep::SheepEntityBundle;
use valence_entity::warden::WardenEntityBundle;
use valence_entity::zombie::ZombieEntityBundle;
use valence_entity::zombie_horse::ZombieHorseEntityBundle;
use valence_entity::{entity, Pose};

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_systems((spawn_entity, intersections))
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block([x, 64, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<(&mut Location, &mut Position, &mut GameMode, &mut Client), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut loc, mut pos, mut game_mode, mut client) in &mut clients {
        loc.0 = instances.single();
        pos.set([0.5, 65.0, 0.5]);
        *game_mode = GameMode::Creative;
        client.send_chat_message("To spawn an entity, press shift. F3 + B to activate hitboxes");
    }
}

fn spawn_entity(
    mut commands: Commands,
    mut sneaking: EventReader<Sneaking>,
    client_query: Query<(&Position, &Location)>,
) {
    for sneaking in sneaking.iter() {
        if sneaking.state == SneakState::Start {
            continue;
        }

        let (position, location) = client_query.get(sneaking.client).unwrap();

        let position = *position;
        let location = *location;

        match rand::thread_rng().gen_range(0..7) {
            0 => commands.spawn(SheepEntityBundle {
                position,
                location,
                entity_name_visible: NameVisible(true),
                ..Default::default()
            }),
            1 => commands.spawn(PigEntityBundle {
                position,
                location,
                entity_name_visible: NameVisible(true),
                ..Default::default()
            }),
            2 => commands.spawn(ZombieEntityBundle {
                position,
                location,
                entity_name_visible: NameVisible(true),
                ..Default::default()
            }),
            3 => commands.spawn(ZombieHorseEntityBundle {
                position,
                location,
                entity_name_visible: NameVisible(true),
                ..Default::default()
            }),
            4 => commands.spawn(WardenEntityBundle {
                position,
                location,
                entity_name_visible: NameVisible(true),
                entity_pose: entity::Pose(Pose::Digging),
                ..Default::default()
            }),
            5 => commands.spawn(WardenEntityBundle {
                position,
                location,
                entity_name_visible: NameVisible(true),
                ..Default::default()
            }),
            6 => commands.spawn(HoglinEntityBundle {
                position,
                location,
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
        let Ok(mut name) = name_query.get_mut(entity) else { continue; };
        name.0 = Some(format!("{value}").into());
    }
}
