use valence_new::client::despawn_disconnected_clients;
use valence_new::client::event::{
    default_event_handler, ChatCommand, FinishDigging, StartDigging, UseItemOnBlock,
};
use valence_new::prelude::*;
use valence_protocol::types::Hand;

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_system_to_stage(EventLoop, interpret_command)
        .add_system_to_stage(EventLoop, digging_creative_mode)
        .add_system_to_stage(EventLoop, digging_survival_mode)
        .add_system_to_stage(EventLoop, place_blocks)
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
            instance.set_block_state([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    world.spawn(instance);
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    let instance = instances.get_single().unwrap();

    for mut client in &mut clients {
        client.set_position([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        client.set_instance(instance);
        client.set_game_mode(GameMode::Creative);
        client.set_op_level(4); // required to use F3+F4, eg /gamemode
        client.send_message(
            "Welcome to Valence! Build something cool. Use F3+F4 to change gamemode".italic(),
        );
    }
}

fn interpret_command(mut clients: Query<&mut Client>, mut events: EventReader<ChatCommand>) {
    for event in events.iter() {
        let Ok(mut client) = clients.get_component_mut::<Client>(event.client) else {
            continue;
        };

        let mut args = event.command.split_whitespace();
        let command = args.next().unwrap_or_default();

        match command {
            "gamemode" => {
                let mode = args.next().unwrap_or_default();
                let mode = match mode {
                    "adventure" => GameMode::Adventure,
                    "creative" => GameMode::Creative,
                    "survival" => GameMode::Survival,
                    "spectator" => GameMode::Spectator,
                    _ => {
                        client.send_message(
                            "Invalid gamemode. Use /gamemode creative or /gamemode survival."
                                .italic(),
                        );
                        continue;
                    }
                };
                client.set_game_mode(mode);
                client.send_message(format!("Set gamemode to {:?}.", mode).italic());
            }
            _ => { /* ignored */ }
        }
    }
}

fn digging_creative_mode(
    clients: Query<&Client>,
    mut instances: Query<&mut Instance>,
    mut events: EventReader<StartDigging>,
) {
    let mut instance = instances.single_mut();

    for event in events.iter() {
        let Ok(client) = clients.get_component::<Client>(event.client) else {
            continue;
        };
        if client.game_mode() == GameMode::Creative {
            instance.set_block_state(event.position, BlockState::AIR);
        }
    }
}

fn digging_survival_mode(
    clients: Query<&Client>,
    mut instances: Query<&mut Instance>,
    mut events: EventReader<FinishDigging>,
) {
    let mut instance = instances.single_mut();

    for event in events.iter() {
        let Ok(client) = clients.get_component::<Client>(event.client) else {
            continue;
        };
        if client.game_mode() == GameMode::Survival {
            instance.set_block_state(event.position, BlockState::AIR);
        }
    }
}

fn place_blocks(
    mut clients: Query<(&Client, &mut Inventory)>,
    mut instances: Query<&mut Instance>,
    mut events: EventReader<UseItemOnBlock>,
) {
    let mut instance = instances.single_mut();

    for event in events.iter() {
        let Ok((client, mut inventory)) = clients.get_mut(event.client) else {
            continue;
        };
        if event.hand != Hand::Main {
            continue;
        }

        // get the held item
        let slot_id = client.held_item_slot();
        let Some(stack) = inventory.slot(slot_id) else {
            // no item in the slot
            continue;
        };

        let Some(block_kind) = stack.item.to_block_kind() else {
            // can't place this item as a block
            continue;
        };

        if client.game_mode() == GameMode::Survival {
            // check if the player has the item in their inventory and remove
            // it.
            let slot = if stack.count() > 1 {
                let mut stack = stack.clone();
                stack.set_count(stack.count() - 1);
                Some(stack)
            } else {
                None
            };
            inventory.replace_slot(slot_id, slot);
        }
        let real_pos = event.position.get_in_direction(event.face);
        instance.set_block_state(real_pos, block_kind.to_state());
    }
}
