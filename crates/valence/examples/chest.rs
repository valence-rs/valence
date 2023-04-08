#![allow(clippy::type_complexity)]

use tracing::warn;
use valence::client::misc::{PlayerInteractBlock, StartSneaking};
use valence::client::{default_event_handler, despawn_disconnected_clients};
use valence::entity::player::PlayerEntityBundle;
use valence::prelude::*;

const SPAWN_Y: i32 = 64;
const CHEST_POS: [i32; 3] = [0, SPAWN_Y + 1, 3];

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_systems(
            (default_event_handler, toggle_gamemode_on_sneak, open_chest)
                .in_schedule(EventLoopSchedule),
        )
        .add_systems(PlayerList::default_systems())
        .add_system(despawn_disconnected_clients)
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
            instance.set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }
    instance.set_block(CHEST_POS, BlockState::CHEST);

    commands.spawn(instance);

    let inventory = Inventory::with_title(
        InventoryKind::Generic9x3,
        "Extra".italic() + " Chesty".not_italic().bold().color(Color::RED) + " Chest".not_italic(),
    );
    commands.spawn(inventory);
}

fn init_clients(
    mut clients: Query<(Entity, &UniqueId, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
    mut commands: Commands,
) {
    for (entity, uuid, mut game_mode) in &mut clients {
        *game_mode = GameMode::Creative;

        commands.entity(entity).insert(PlayerEntityBundle {
            location: Location(instances.single()),
            position: Position::new([0.5, SPAWN_Y as f64 + 1.0, 0.5]),
            uuid: *uuid,
            ..Default::default()
        });
    }
}

fn toggle_gamemode_on_sneak(
    mut clients: Query<&mut GameMode>,
    mut events: EventReader<StartSneaking>,
) {
    for event in events.iter() {
        let Ok(mut mode) = clients.get_mut(event.client) else {
            continue;
        };
        *mode = match *mode {
            GameMode::Survival => GameMode::Creative,
            GameMode::Creative => GameMode::Survival,
            _ => GameMode::Creative,
        };
    }
}

fn open_chest(
    mut commands: Commands,
    inventories: Query<Entity, (With<Inventory>, Without<Client>)>,
    mut events: EventReader<PlayerInteractBlock>,
) {
    let Ok(inventory) = inventories.get_single() else {
        warn!("No inventories");
        return;
    };

    for event in events.iter() {
        if event.position != CHEST_POS.into() {
            continue;
        }
        let open_inventory = OpenInventory::new(inventory);
        commands.entity(event.client).insert(open_inventory);
    }
}
