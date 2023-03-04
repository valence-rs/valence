use std::path::PathBuf;

use valence::prelude::*;
use valence_client::misc::InteractBlock;
use valence_inventory::ClientInventoryState;
use valence_nbt::compound;
use valence_schem::Schematic;

const FLOOR_Y: i32 = 64;
const SPAWN_POS: DVec3 = DVec3::new(0.5, FLOOR_Y as f64 + 1.0, 0.5);

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_systems((
            init_clients,
            first_pos,
            second_pos,
            origin,
            copy_schem,
            paste_schem,
            save_schem,
            place_blocks,
            break_blocks,
        ))
        .add_system(despawn_disconnected_clients)
        .run();
}

#[derive(Debug, Clone, Copy, Component)]
struct FirstPos(BlockPos);
#[derive(Debug, Clone, Copy, Component)]
struct SecondPos(BlockPos);
#[derive(Debug, Clone, Copy, Component)]
struct Origin(BlockPos);
#[derive(Debug, Clone, Component)]
struct Clipboard(Schematic);

fn first_pos(
    mut clients: Query<(
        &mut Client,
        &Inventory,
        Option<&mut FirstPos>,
        &ClientInventoryState,
    )>,
    mut block_breaks: EventReader<Digging>,
    mut commands: Commands,
) {
    for Digging {
        client: entity,
        position,
        ..
    } in block_breaks.iter()
    {
        let Ok((mut client, inv, pos, inv_state)) = clients.get_mut(*entity) else {
            continue;
        };
        let slot = inv.slot(inv_state.held_item_slot());
        if matches!(slot, Some(ItemStack {item, ..}) if *item == ItemKind::WoodenAxe) {
            let changed = !matches!(pos.map(|pos| pos.0), Some(pos) if pos == *position);
            if changed {
                client.send_message(format!(
                    "Set the primary pos to ({}, {}, {})",
                    position.x, position.y, position.z,
                ));
            }
            commands.entity(*entity).insert(FirstPos(*position));
        }
    }
}

fn second_pos(
    mut clients: Query<(
        &mut Client,
        &Inventory,
        Option<&mut SecondPos>,
        &ClientInventoryState,
    )>,
    mut interacts: EventReader<InteractBlock>,
    mut commands: Commands,
) {
    for InteractBlock {
        client: entity,
        hand,
        position,
        ..
    } in interacts.iter()
    {
        if *hand == Hand::Main {
            let Ok((mut client, inv, pos, inv_state)) = clients.get_mut(*entity) else {
                continue;
            };
            let slot = inv.slot(inv_state.held_item_slot());
            if matches!(slot, Some(ItemStack {item, ..}) if *item == ItemKind::WoodenAxe) {
                let changed = !matches!(pos.map(|pos| pos.0), Some(pos) if pos == *position);
                if changed {
                    client.send_message(format!(
                        "Set the secondary pos to ({}, {}, {})",
                        position.x, position.y, position.z,
                    ));
                }
                commands.entity(*entity).insert(SecondPos(*position));
            }
        }
    }
}

fn origin(
    mut clients: Query<(
        &mut Client,
        &Inventory,
        Option<&mut Origin>,
        &ClientInventoryState,
    )>,
    mut interacts: EventReader<InteractBlock>,
    mut commands: Commands,
) {
    for InteractBlock {
        client: entity,
        hand,
        position,
        ..
    } in interacts.iter()
    {
        if *hand == Hand::Main {
            let Ok((mut client, inv, pos, inv_state)) = clients.get_mut(*entity) else {
                continue;
            };
            let slot = inv.slot(inv_state.held_item_slot());
            if matches!(slot, Some(ItemStack {item, ..}) if *item == ItemKind::Stick) {
                let changed = !matches!(pos.map(|pos| pos.0), Some(pos) if pos == *position);
                if changed {
                    client.send_message(format!(
                        "Set the origin to ({}, {}, {})",
                        position.x, position.y, position.z,
                    ));
                }
                commands.entity(*entity).insert(Origin(*position));
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn copy_schem(
    mut clients: Query<(
        &mut Client,
        &Inventory,
        Option<&FirstPos>,
        Option<&SecondPos>,
        Option<&Origin>,
        &ClientInventoryState,
        &Position,
        &Location,
        &Username,
    )>,
    instances: Query<&Instance>,
    mut interacts: EventReader<InteractBlock>,
    biome_registry: Res<BiomeRegistry>,
    biomes: Query<&Biome>,
    mut commands: Commands,
) {
    for InteractBlock {
        client: entity,
        hand,
        ..
    } in interacts.iter()
    {
        if *hand != Hand::Main {
            continue;
        }
        let Ok((mut client, inv, pos1, pos2, origin, inv_state, &Position(pos), &Location(instance), Username(username))) = clients.get_mut(*entity) else {
            continue;
        };
        let slot = inv.slot(inv_state.held_item_slot());
        if matches!(slot, Some(ItemStack {item, ..}) if *item == ItemKind::Paper) {
            let Some((FirstPos(pos1), SecondPos(pos2))) = pos1.zip(pos2) else {
                client.send_message("Specify both positions first");
                continue;
            };
            let origin = origin.map(|pos| pos.0).unwrap_or(BlockPos::at(pos));

            let Ok(instance) = instances.get(instance) else {
                continue;
            };
            let mut schematic = Schematic::copy(instance, (*pos1, *pos2), origin, |id| {
                let biome = biome_registry.get_by_id(id).unwrap();
                biomes.get(biome).unwrap().name.clone()
            });
            schematic.metadata.replace(compound! {"Author" => username});
            commands.entity(*entity).insert(Clipboard(schematic));
            client.send_message("Copied");
        }
    }
}

fn paste_schem(
    mut instances: Query<&mut Instance>,
    mut clients: Query<(
        &mut Client,
        &Inventory,
        Option<&Clipboard>,
        &Location,
        &ClientInventoryState,
        &Position,
    )>,
    mut interacts: EventReader<InteractBlock>,
) {
    for InteractBlock {
        client: entity,
        hand,
        ..
    } in interacts.iter()
    {
        if *hand != Hand::Main {
            continue;
        }
        let Ok((mut client, inv, clipboard, &Location(instance), inv_state, &Position(position))) = clients.get_mut(*entity) else {
            continue;
        };
        let Ok(mut instance) = instances.get_mut(instance) else {
            continue;
        };
        let slot = inv.slot(inv_state.held_item_slot());
        if matches!(slot, Some(ItemStack {item, ..}) if *item == ItemKind::Feather) {
            let Some(Clipboard(schematic)) = clipboard else {
                client.send_message("Copy something to clipboard first!");
                continue;
            };
            let pos = BlockPos::at(position);
            schematic.paste(&mut instance, pos, |_| BiomeId::default());
            client.send_message(format!(
                "Pasted schematic at ({} {} {})",
                pos.x, pos.y, pos.z
            ));
        }
    }
}

fn save_schem(
    mut clients: Query<(
        &mut Client,
        &Inventory,
        Option<&Clipboard>,
        &ClientInventoryState,
        &Username,
    )>,
    mut interacts: EventReader<InteractBlock>,
) {
    for InteractBlock {
        client: entity,
        hand,
        ..
    } in interacts.iter()
    {
        if *hand != Hand::Main {
            continue;
        }
        let Ok((mut client, inv, clipboard, inv_state, Username(username))) = clients.get_mut(*entity) else {
            continue;
        };
        let slot = inv.slot(inv_state.held_item_slot());
        if matches!(slot, Some(ItemStack {item, ..}) if *item == ItemKind::MusicDiscStal) {
            let Some(Clipboard(schematic)) = clipboard else {
                client.send_message("Copy something to clipboard first!");
                continue;
            };
            let path = PathBuf::from(format!("{username}.schem"));
            schematic.save(&path).unwrap();
            client.send_message(format!("Saved schem to {}", path.display()));
        }
    }
}

fn place_blocks(
    clients: Query<(&Inventory, &ClientInventoryState), With<Client>>,
    mut instances: Query<&mut Instance>,
    mut events: EventReader<InteractBlock>,
) {
    let mut instance = instances.single_mut();

    for event in events.iter() {
        let Ok((inventory, inv_state)) = clients.get(event.client) else {
            continue;
        };
        if event.hand != Hand::Main {
            continue;
        }

        let Some(stack) = inventory.slot(inv_state.held_item_slot()) else {
            continue;
        };

        let Some(block_kind) = BlockKind::from_item_kind(stack.item) else {
            continue;
        };

        let pos = event.position.get_in_direction(event.face);
        instance.set_block(pos, block_kind.to_state());
    }
}

fn break_blocks(
    mut instances: Query<&mut Instance>,
    inventories: Query<(&Inventory, &ClientInventoryState)>,
    mut events: EventReader<Digging>,
) {
    let mut instance = instances.single_mut();

    for Digging {
        client, position, ..
    } in events.iter()
    {
        let Ok((inv, inv_state)) = inventories.get(*client) else {
            continue;
        };

        let slot = inv.slot(inv_state.held_item_slot());
        if !matches!(slot, Some(ItemStack {item, ..}) if *item == ItemKind::WoodenAxe) {
            instance.set_block(*position, BlockState::AIR);
        }
    }
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Query<&DimensionType>,
    biomes: Query<&Biome>,
) {
    let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

    for x in -16..=16 {
        for z in -16..=16 {
            let pos = BlockPos::new(SPAWN_POS.x as i32 + x, FLOOR_Y, SPAWN_POS.z as i32 + z);
            instance
                .chunk_entry(ChunkPos::from_block_pos(pos))
                .or_default();
            instance.set_block(pos, BlockState::QUARTZ_BLOCK);
        }
    }

    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<
        (&mut Inventory, &mut Location, &mut Position, &mut GameMode),
        Added<Client>,
    >,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut inv, mut loc, mut pos, mut game_mode) in &mut clients {
        *game_mode = GameMode::Creative;
        loc.0 = instances.single();
        pos.set(SPAWN_POS);

        inv.set_slot(
            36,
            Some(ItemStack::new(
                ItemKind::WoodenAxe,
                1,
                Some(compound! {"display" => compound! {"Name" => "Position Setter".not_italic()}}),
            )),
        );
        inv.set_slot(
            37,
            Some(ItemStack::new(
                ItemKind::Stick,
                1,
                Some(compound! {"display" => compound! {"Name" => "Origin Setter".not_italic()}}),
            )),
        );
        inv.set_slot(
            38,
            Some(ItemStack::new(
                ItemKind::Paper,
                1,
                Some(compound! {"display" => compound! {"Name" => "Copy Schematic".not_italic()}}),
            )),
        );
        inv.set_slot(
            39,
            Some(ItemStack::new(
                ItemKind::Feather,
                1,
                Some(compound! {"display" => compound! {"Name" => "Paste Schematic".not_italic()}}),
            )),
        );
        inv.set_slot(
            40,
            Some(ItemStack::new(
                ItemKind::MusicDiscStal,
                1,
                Some(compound! {"display" => compound! {"Name" => "Save Schematic".not_italic().color(Color::WHITE)}}),
            )),
        );
    }
}
