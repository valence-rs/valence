use tracing::warn;
use valence::client::despawn_disconnected_clients;
use valence::client::event::{default_event_handler, StartSneaking, UseItemOnBlock};
use valence::prelude::*;

const SPAWN_Y: i32 = 64;
const CHEST_POS: [i32; 3] = [0, SPAWN_Y + 1, 3];

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_system_to_stage(EventLoop, toggle_gamemode_on_sneak)
        .add_system_to_stage(EventLoop, open_chest)
        .add_system_set(PlayerList::default_system_set())
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
            instance.set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }
    instance.set_block(CHEST_POS, BlockState::CHEST);

    world.spawn(instance);

    let inventory = Inventory::with_title(
        InventoryKind::Generic9x3,
        "Extra".italic() + " Chesty".not_italic().bold().color(Color::RED) + " Chest".not_italic(),
    );
    world.spawn(inventory);
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for mut client in &mut clients {
        client.set_position([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        client.set_instance(instances.single());
        client.set_game_mode(GameMode::Creative);
    }
}

fn toggle_gamemode_on_sneak(
    mut clients: Query<&mut Client>,
    mut events: EventReader<StartSneaking>,
) {
    for event in events.iter() {
        let Ok(mut client) = clients.get_component_mut::<Client>(event.client) else {
            continue;
        };
        let mode = client.game_mode();
        client.set_game_mode(match mode {
            GameMode::Survival => GameMode::Creative,
            GameMode::Creative => GameMode::Survival,
            _ => GameMode::Creative,
        });
    }
}

fn open_chest(
    mut commands: Commands,
    inventories: Query<Entity, (With<Inventory>, Without<Client>)>,
    mut events: EventReader<UseItemOnBlock>,
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
