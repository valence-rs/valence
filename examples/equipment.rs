#![allow(clippy::type_complexity)]

const SPAWN_Y: i32 = 64;

use rand::Rng;
use valence::entity::zombie::ZombieEntityBundle;
use valence::prelude::*;
use valence_inventory::player_inventory::PlayerInventory;
use valence_inventory::HeldItem;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (despawn_disconnected_clients,))
        .add_systems(
            Update,
            (
                init_clients,
                despawn_disconnected_clients,
                randomize_equipment,
                update_player_inventory,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            layer
                .chunk
                .set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(layer);
}

fn init_clients(
    mut clients: Query<
        (
            &mut Position,
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
    mut commands: Commands,
) {
    for (
        mut pos,
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut game_mode,
    ) in &mut clients
    {
        let layer = layers.single();

        pos.0 = [0.0, f64::from(SPAWN_Y) + 1.0, 0.0].into();
        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        *game_mode = GameMode::Survival;

        commands
            .spawn(ZombieEntityBundle {
                position: *pos,
                layer: *layer_id,
                ..Default::default()
            })
            .insert(Equipment::default());
    }
}

fn randomize_equipment(mut query: Query<&mut Equipment>, server: Res<Server>) {
    let ticks = server.current_tick() as u32;
    // every second
    if ticks % server.tick_rate() != 0 {
        return;
    }

    for mut equipment in &mut query {
        equipment.clear();

        tracing::info!("Randomizing equipment for entity");

        let (slot, item_stack) = match rand::thread_rng().gen_range(0..=5) {
            0 => (
                Equipment::MAIN_HAND_IDX,
                ItemStack::new(ItemKind::DiamondSword, 1, None),
            ),
            1 => (
                Equipment::OFF_HAND_IDX,
                ItemStack::new(ItemKind::Shield, 1, None),
            ),
            2 => (
                Equipment::BOOTS_IDX,
                ItemStack::new(ItemKind::DiamondBoots, 1, None),
            ),
            3 => (
                Equipment::LEGGINGS_IDX,
                ItemStack::new(ItemKind::DiamondLeggings, 1, None),
            ),
            4 => (
                Equipment::CHESTPLATE_IDX,
                ItemStack::new(ItemKind::DiamondChestplate, 1, None),
            ),
            5 => (
                Equipment::HELMET_IDX,
                ItemStack::new(ItemKind::DiamondHelmet, 1, None),
            ),
            _ => unreachable!(),
        };

        equipment.set_slot(slot, item_stack);
    }
}

/// Updating the Equipment will only be visible for other players,
/// so we need to update the player's inventory as well.
fn update_player_inventory(
    mut clients: Query<(&mut Inventory, &Equipment, &HeldItem), Changed<Equipment>>,
) {
    for (mut inv, equipment, held_item) in &mut clients {
        inv.set_slot(PlayerInventory::SLOT_HEAD, equipment.helmet().clone());
        inv.set_slot(PlayerInventory::SLOT_CHEST, equipment.chestplate().clone());
        inv.set_slot(PlayerInventory::SLOT_LEGS, equipment.leggings().clone());
        inv.set_slot(PlayerInventory::SLOT_FEET, equipment.boots().clone());
        inv.set_slot(PlayerInventory::SLOT_OFFHAND, equipment.off_hand().clone());
        inv.set_slot(held_item.slot(), equipment.main_hand().clone());
    }
}
