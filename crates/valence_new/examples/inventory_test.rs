use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ShouldRun;
use tracing::info;
use valence_new::client::Client;
use valence_new::config::{Config, ConnectionMode};
use valence_new::dimension::DimensionId;
use valence_new::inventory::{Inventory, InventoryKind, OpenInventory};
use valence_new::protocol::types::GameMode;
use valence_new::server::Server;
use valence_protocol::{ItemKind, ItemStack};

#[derive(Resource)]
struct GameState {
    instance: Entity,
    inventory: usize,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    valence_new::run_server(
        Config::default().with_connection_mode(ConnectionMode::Offline),
        SystemStage::parallel()
            .with_system(setup.with_run_criteria(ShouldRun::once))
            .with_system(init_clients)
            .with_system(open_inventory_test)
            .with_system(blink_items),
        (),
    )
}

fn setup(world: &mut World) {
    let instance = world
        .resource::<Server>()
        .new_instance(DimensionId::default());

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
