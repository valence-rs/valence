#![allow(clippy::type_complexity)]

use valence::client::misc::InteractBlock;
use valence::client::ClientInventoryState;
use valence::prelude::*;
use valence::protocol::types::Hand;

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_systems((
            toggle_gamemode_on_sneak,
            digging_creative_mode,
            digging_survival_mode,
            place_blocks,
        ))
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

    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<(&mut Client, &mut Location, &mut Position, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut client, mut loc, mut pos, mut game_mode) in &mut clients {
        *game_mode = GameMode::Creative;
        loc.0 = instances.single();
        pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);

        client.send_message("Welcome to Valence! Build something cool.".italic());
    }
}

fn toggle_gamemode_on_sneak(mut clients: Query<&mut GameMode>, mut events: EventReader<Sneaking>) {
    for event in events.iter() {
        let Ok(mut mode) = clients.get_mut(event.client) else {
            continue;
        };
        if event.state == SneakState::Start {
            *mode = match *mode {
                GameMode::Survival => GameMode::Creative,
                GameMode::Creative => GameMode::Survival,
                _ => GameMode::Creative,
            };
        }
    }
}

fn digging_creative_mode(
    clients: Query<&GameMode>,
    mut instances: Query<&mut Instance>,
    mut events: EventReader<Digging>,
) {
    let mut instance = instances.single_mut();

    for event in events.iter() {
        let Ok(game_mode) = clients.get(event.client) else {
            continue;
        };
        if *game_mode == GameMode::Creative && event.state == DiggingState::Start {
            instance.set_block(event.position, BlockState::AIR);
        }
    }
}

fn digging_survival_mode(
    clients: Query<&GameMode>,
    mut instances: Query<&mut Instance>,
    mut events: EventReader<Digging>,
) {
    let mut instance = instances.single_mut();

    for event in events.iter() {
        let Ok(game_mode) = clients.get(event.client) else {
            continue;
        };
        if *game_mode == GameMode::Survival && event.state == DiggingState::Stop {
            instance.set_block(event.position, BlockState::AIR);
        }
    }
}

fn place_blocks(
    mut clients: Query<(&mut Inventory, &GameMode, &ClientInventoryState)>,
    mut instances: Query<&mut Instance>,
    mut events: EventReader<InteractBlock>,
) {
    let mut instance = instances.single_mut();

    for event in events.iter() {
        let Ok((mut inventory, game_mode, inv_state)) = clients.get_mut(event.client) else {
            continue;
        };
        if event.hand != Hand::Main {
            continue;
        }

        // get the held item
        let slot_id = inv_state.held_item_slot();
        let Some(stack) = inventory.slot(slot_id) else {
            // no item in the slot
            continue;
        };

        let Some(block_kind) = stack.item.to_block_kind() else {
            // can't place this item as a block
            continue;
        };

        if *game_mode == GameMode::Survival {
            // check if the player has the item in their inventory and remove
            // it.
            if stack.count() > 1 {
                let count = stack.count();
                inventory.set_slot_amount(slot_id, count - 1);
            } else {
                inventory.set_slot(slot_id, None);
            }
        }
        let real_pos = event.position.get_in_direction(event.face);
        instance.set_block(real_pos, block_kind.to_state());
    }
}
