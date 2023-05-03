use std::collections::HashMap;

use bevy_app::App;
use bevy_ecs::prelude::Entity;
use valence::prelude::*;
use valence_entity::sheep::{self, SheepEntityBundle};

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_systems((spawn_sheep, intersections))
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Query<&DimensionType>,
    biomes: Query<&Biome>,
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
        client.send_message(
            "To spawn a sheep sneak. The color of sheep depends on their count of intersections \
             with other hitboxes (with your also). Use F3 + B to activate hitboxes",
        );
    }
}

fn spawn_sheep(
    mut commands: Commands,
    mut sneaking: EventReader<Sneaking>,
    client_query: Query<(&Position, &Location)>,
) {
    for sneaking in sneaking.iter() {
        if sneaking.state == SneakState::Start {
            continue;
        }

        let (pos, loc) = client_query.get(sneaking.client).unwrap();

        commands.spawn(SheepEntityBundle {
            location: *loc,
            position: *pos,
            ..Default::default()
        });
    }
}

fn intersections(
    query: Query<(Entity, &Hitbox, &Position)>,
    mut sheep_color_query: Query<&mut sheep::Color>,
) {
    // This code only to show how hitboxes can be used
    let mut intersections = HashMap::new();

    for [(entity1, hitbox1, pos1), (entity2, hitbox2, pos2)] in query.iter_combinations() {
        let aabb1 = hitbox1.in_world(pos1.0);
        let aabb2 = hitbox2.in_world(pos2.0);

        let _ = *intersections.entry(entity1).or_insert(0);
        let _ = *intersections.entry(entity2).or_insert(0);

        if aabb1.intersects(aabb2) {
            *intersections.get_mut(&entity1).unwrap() += 1;
            *intersections.get_mut(&entity2).unwrap() += 1;
        }
    }

    for (entity, value) in intersections {
        let Ok(mut color) = sheep_color_query.get_mut(entity) else { continue; };
        color.0 = value;
    }
}
