use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ShouldRun;
use tracing::info;
use valence_new::client::event::{default_event_handler, StartSneaking, UseItemOnBlock};
use valence_new::client::{despawn_disconnected_clients, Client};
use valence_new::config::{Config, ConnectionMode};
use valence_new::dimension::DimensionId;
use valence_new::instance::Chunk;
use valence_new::inventory::{Inventory, InventoryKind, OpenInventory};
use valence_new::protocol::types::GameMode;
use valence_new::server::Server;
use valence_protocol::{BlockState, ItemKind, ItemStack};

#[derive(Resource)]
struct GameState {
    instance: Entity,
    inventory: usize,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    valence_new::run_server(
        Config::default().with_connection_mode(ConnectionMode::Offline),
        SystemStage::parallel()
            .with_system(setup.with_run_criteria(ShouldRun::once))
            .with_system(init_clients)
            .with_system(default_event_handler())
            .with_system(despawn_disconnected_clients)
            // .with_system(open_inventory_test)
            // .with_system(blink_items)
            .with_system(open_inventory_on_interact)
            .with_system(toggle_gamemode_on_sneak),
        (),
    )
}

fn setup(world: &mut World) {
    let mut instance = world
        .resource::<Server>()
        .new_instance(DimensionId::default());

    // Create spawn platform.
    for z in -5..5 {
        for x in -5..5 {
            let mut chunk = Chunk::new(24);
            for z in 0..16 {
                for x in 0..16 {
                    chunk.set_block_state(x, 10, z, BlockState::STONE);
                }
            }

            if x == 0 && z == 0 {
                for sx in 0..3 {
                    for sz in 0..3 {
                        chunk.set_block_state(sx, 10, sz, BlockState::BRICKS);
                    }
                    chunk.set_block_state(sx, 11, 0, BlockState::CHEST);
                }
                chunk.set_block_state(0, 10, 0, BlockState::COPPER_BLOCK);
                chunk.set_block_state(1, 10, 0, BlockState::IRON_BLOCK);
                chunk.set_block_state(2, 10, 0, BlockState::GOLD_BLOCK);
            }

            instance.insert_chunk([x, z], chunk);
        }
    }

    let id = world.spawn(instance).id();
    world.insert_resource(GameState {
        instance: id,
        inventory: 0,
    });

    // create inventories to view
    let mut inventories = [
        Inventory::new(InventoryKind::Generic9x2),
        Inventory::new(InventoryKind::Generic9x3),
        Inventory::new(InventoryKind::Crafting),
    ];

    for mut inv in inventories {
        inv.replace_slot(0, Some(ItemStack::new(ItemKind::DiamondPickaxe, 1, None)));
        world.spawn(inv);
    }
}

fn init_clients(mut clients: Query<&mut Client, Added<Client>>, state: Res<GameState>) {
    for mut client in &mut clients {
        client.set_instance(state.instance);
        client.set_game_mode(GameMode::Creative);
    }
}

fn open_inventory_test(
    mut state: ResMut<GameState>,
    mut commands: Commands,
    clients: Query<(Entity, With<Client>, Without<OpenInventory>)>,
    inventories: Query<(Entity, With<Inventory>, Without<Client>)>,
) {
    if clients.is_empty() {
        return;
    }
    if state.inventory >= inventories.iter().count() {
        state.inventory = 0;
    }
    let (target_inventory, _, _) = inventories.iter().skip(state.inventory).next().unwrap();
    info!("opening inventory {}", state.inventory);
    for (entity, _, _) in &clients {
        commands
            .entity(entity)
            .insert(OpenInventory::new(target_inventory));
    }
    state.inventory += 1;
}

fn blink_items(mut inventories: Query<&mut Inventory>) {
    for mut inv in inventories.iter_mut() {
        if inv.slot(1).is_some() {
            inv.replace_slot(1, None);
        } else {
            inv.replace_slot(1, Some(ItemStack::new(ItemKind::Diamond, 1, None)));
        }
    }
}

fn open_inventory_on_interact(
    mut commands: Commands,
    inventories: Query<(Entity, With<Inventory>, Without<Client>)>,
    mut events: EventReader<UseItemOnBlock>,
) {
    for event in events.iter() {
        let inventory_idx = event.position.x as usize % 3;
        info!("opening inventory {}", inventory_idx);
        let (target_inventory, _, _) = inventories.iter().skip(inventory_idx).next().unwrap();
        commands
            .entity(event.client)
            .insert(OpenInventory::new(target_inventory));
    }
}

fn toggle_gamemode_on_sneak(
    mut clients: Query<&mut Client>,
    mut events: EventReader<StartSneaking>,
) {
    for event in events.iter() {
        if let Ok(mut client) = clients.get_component_mut::<Client>(event.client) {
            let mode = client.game_mode();
            client.set_game_mode(match mode {
                GameMode::Survival => GameMode::Creative,
                GameMode::Creative => GameMode::Survival,
                _ => GameMode::Creative,
            });
        }
    }
}
