#![allow(clippy::type_complexity)]

use std::path::PathBuf;

use valence::prelude::*;
use valence_client::interact_block::InteractBlockEvent;
use valence_client::message::SendMessage;
use valence_inventory::HeldItem;
use valence_nbt::compound;
use valence_schem::Schematic;

const FLOOR_Y: i32 = 64;
const SPAWN_POS: DVec3 = DVec3::new(0.5, FLOOR_Y as f64 + 1.0, 0.5);

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                init_clients,
                first_pos,
                second_pos,
                origin,
                copy_schem,
                paste_schem,
                save_schem,
                place_blocks,
                break_blocks,
                despawn_disconnected_clients,
            ),
        )
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
    mut clients: Query<(&mut Client, &Inventory, Option<&mut FirstPos>, &HeldItem)>,
    mut block_breaks: EventReader<DiggingEvent>,
    mut commands: Commands,
) {
    for DiggingEvent {
        client: entity,
        position,
        ..
    } in block_breaks.iter()
    {
        let Ok((mut client, inv, pos, held_item)) = clients.get_mut(*entity) else {
            continue;
        };
        let slot = inv.slot(held_item.slot());
        if !matches!(slot, Some(ItemStack {item, ..}) if *item == ItemKind::WoodenAxe) {
            continue;
        }
        let changed = !matches!(pos.map(|pos| pos.0), Some(pos) if pos == *position);
        if changed {
            client.send_chat_message(format!(
                "Set the primary pos to ({}, {}, {})",
                position.x, position.y, position.z,
            ));
        }
        commands.entity(*entity).insert(FirstPos(*position));
    }
}

fn second_pos(
    mut clients: Query<(&mut Client, &Inventory, Option<&mut SecondPos>, &HeldItem)>,
    mut interacts: EventReader<InteractBlockEvent>,
    mut commands: Commands,
) {
    for InteractBlockEvent {
        client: entity,
        hand,
        position,
        ..
    } in interacts.iter()
    {
        if *hand != Hand::Main {
            continue;
        }
        let Ok((mut client, inv, pos, held_item)) = clients.get_mut(*entity) else {
            continue;
        };
        let slot = inv.slot(held_item.slot());
        if !matches!(slot, Some(ItemStack {item, ..}) if *item == ItemKind::WoodenAxe) {
            continue;
        }
        println!("So this is secondary pos");
        let changed = !matches!(pos.map(|pos| pos.0), Some(pos) if pos == *position);
        if changed {
            client.send_chat_message(format!(
                "Set the secondary pos to ({}, {}, {})",
                position.x, position.y, position.z,
            ));
        }
        commands.entity(*entity).insert(SecondPos(*position));
    }
}

fn origin(
    mut clients: Query<(&mut Client, &Inventory, Option<&mut Origin>, &HeldItem)>,
    mut interacts: EventReader<InteractBlockEvent>,
    mut commands: Commands,
) {
    for InteractBlockEvent {
        client: entity,
        hand,
        position,
        ..
    } in interacts.iter()
    {
        if *hand != Hand::Main {
            continue;
        }
        let Ok((mut client, inv, pos, held_item)) = clients.get_mut(*entity) else {
            continue;
        };
        let slot = inv.slot(held_item.slot());
        if !matches!(slot, Some(ItemStack {item, ..}) if *item == ItemKind::Stick) {
            continue;
        }
        let changed = !matches!(pos.map(|pos| pos.0), Some(pos) if pos == *position);
        if changed {
            client.send_chat_message(format!(
                "Set the origin to ({}, {}, {})",
                position.x, position.y, position.z,
            ));
        }
        commands.entity(*entity).insert(Origin(*position));
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
        &HeldItem,
        &Position,
        &VisibleChunkLayer,
        &Username,
    )>,
    layers: Query<&ChunkLayer>,
    mut interacts: EventReader<InteractBlockEvent>,
    biome_registry: Res<BiomeRegistry>,
    mut commands: Commands,
) {
    for InteractBlockEvent {
        client: entity,
        hand,
        ..
    } in interacts.iter()
    {
        if *hand != Hand::Main {
            continue;
        }
        let Ok((
            mut client,
            inv,
            pos1,
            pos2,
            origin,
            held_item,
            &Position(pos),
            &VisibleChunkLayer(layer),
            Username(username),
        )) = clients.get_mut(*entity)
        else {
            continue;
        };
        let slot = inv.slot(held_item.slot());
        if !matches!(slot, Some(ItemStack {item, ..}) if *item == ItemKind::Paper) {
            continue;
        }
        let Some((FirstPos(pos1), SecondPos(pos2))) = pos1.zip(pos2) else {
            client.send_chat_message("Specify both positions first");
            continue;
        };
        let origin = origin.map(|pos| pos.0).unwrap_or(BlockPos::from_pos(pos));

        let Ok(layer) = layers.get(layer) else {
            continue;
        };
        let mut schematic = Schematic::copy(layer, (*pos1, *pos2), origin, |id| {
            biome_registry
                .iter()
                .find(|biome| biome.0 == id)
                .unwrap()
                .1
                .to_string_ident()
        });
        schematic.metadata.replace(compound! {"Author" => username});
        commands.entity(*entity).insert(Clipboard(schematic));
        client.send_chat_message("Copied");
    }
}

fn paste_schem(
    mut layers: Query<&mut ChunkLayer>,
    mut clients: Query<(
        &mut Client,
        &Inventory,
        Option<&Clipboard>,
        &VisibleChunkLayer,
        &HeldItem,
        &Position,
    )>,
    mut interacts: EventReader<InteractBlockEvent>,
) {
    for InteractBlockEvent {
        client: entity,
        hand,
        ..
    } in interacts.iter()
    {
        if *hand != Hand::Main {
            continue;
        }
        let Ok((
            mut client,
            inv,
            clipboard,
            &VisibleChunkLayer(layer),
            held_item,
            &Position(position),
        )) = clients.get_mut(*entity)
        else {
            continue;
        };
        let Ok(mut instance) = layers.get_mut(layer) else {
            continue;
        };
        let slot = inv.slot(held_item.slot());
        if !matches!(slot, Some(ItemStack {item, ..}) if *item == ItemKind::Feather) {
            continue;
        }
        let Some(Clipboard(schematic)) = clipboard else {
            client.send_chat_message("Copy something to clipboard first!");
            continue;
        };
        let pos = BlockPos::from_pos(position);
        schematic.paste(&mut instance, pos, |_| BiomeId::default());
        client.send_chat_message(format!(
            "Pasted schematic at ({} {} {})",
            pos.x, pos.y, pos.z
        ));
    }
}

fn save_schem(
    mut clients: Query<(
        &mut Client,
        &Inventory,
        Option<&Clipboard>,
        &HeldItem,
        &Username,
    )>,
    mut interacts: EventReader<InteractBlockEvent>,
) {
    for InteractBlockEvent {
        client: entity,
        hand,
        ..
    } in interacts.iter()
    {
        if *hand != Hand::Main {
            continue;
        }
        let Ok((mut client, inv, clipboard, held_item, Username(username))) =
            clients.get_mut(*entity)
        else {
            continue;
        };
        let slot = inv.slot(held_item.slot());
        if !matches!(slot, Some(ItemStack {item, ..}) if *item == ItemKind::MusicDiscStal) {
            continue;
        }
        let Some(Clipboard(schematic)) = clipboard else {
            client.send_chat_message("Copy something to clipboard first!");
            continue;
        };
        let path = PathBuf::from(format!("{username}.schem"));
        schematic.save(&path).unwrap();
        client.send_chat_message(format!("Saved schem to {}", path.display()));
    }
}

fn place_blocks(
    clients: Query<(&Inventory, &HeldItem), With<Client>>,
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<InteractBlockEvent>,
) {
    let mut layer = layers.single_mut();

    for event in events.iter() {
        let Ok((inventory, held_item)) = clients.get(event.client) else {
            continue;
        };
        if event.hand != Hand::Main {
            continue;
        }

        let Some(stack) = inventory.slot(held_item.slot()) else {
            continue;
        };

        let Some(block_kind) = BlockKind::from_item_kind(stack.item) else {
            continue;
        };

        let pos = event.position.get_in_direction(event.face);
        layer.set_block(pos, block_kind.to_state());
    }
}

fn break_blocks(
    mut layers: Query<&mut ChunkLayer>,
    inventories: Query<(&Inventory, &HeldItem)>,
    mut events: EventReader<DiggingEvent>,
) {
    let mut layer = layers.single_mut();

    for DiggingEvent {
        client, position, ..
    } in events.iter()
    {
        let Ok((inv, held_item)) = inventories.get(*client) else {
            continue;
        };

        let slot = inv.slot(held_item.slot());
        if !matches!(slot, Some(ItemStack {item, ..}) if *item == ItemKind::WoodenAxe) {
            layer.set_block(*position, BlockState::AIR);
        }
    }
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    for x in -16..=16 {
        for z in -16..=16 {
            let pos = BlockPos::new(SPAWN_POS.x as i32 + x, FLOOR_Y, SPAWN_POS.z as i32 + z);
            layer
                .chunk
                .chunk_entry(ChunkPos::from_block_pos(pos))
                .or_default();
            layer.chunk.set_block(pos, BlockState::QUARTZ_BLOCK);
        }
    }

    commands.spawn(layer);
}

fn init_clients(
    mut clients: Query<
        (
            &mut Inventory,
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
        mut inv,
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
        pos.set(SPAWN_POS);
        *game_mode = GameMode::Creative;

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
