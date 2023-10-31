#![allow(clippy::type_complexity)]

use valence::prelude::*;
use valence_inventory::player_inventory::PlayerInventory;

const SPAWN_Y: i32 = 0;
const SIZE: i32 = 5;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (init_clients, despawn_disconnected_clients))
        .run();
}

fn setup(
    mut commands: Commands,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    server: Res<Server>,
) {
    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -SIZE..SIZE {
        for x in -SIZE..SIZE {
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    for x in -SIZE * 16..SIZE * 16 {
        for z in -SIZE * 16..SIZE * 16 {
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
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut GameMode,
            &mut Inventory,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for (
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut game_mode,
        mut inventory,
    ) in &mut clients
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        *game_mode = GameMode::Survival;

        inventory.set_slot(
            PlayerInventory::SLOT_HEAD,
            ItemStack::new(ItemKind::Glass, 1, None),
        );
        inventory.set_slot(
            PlayerInventory::hotbar_to_slot(4),
            ItemStack::new(ItemKind::Compass, 1, None),
        );
        inventory.set_slot(
            PlayerInventory::SLOT_OFFHAND,
            ItemStack::new(ItemKind::Shield, 1, None),
        );
        inventory.set_slot(
            PlayerInventory::SLOT_OFFHAND,
            ItemStack::new(ItemKind::Shield, 1, None),
        );

        for slot in PlayerInventory::SLOTS_CRAFT_INPUT {
            inventory.set_slot(slot, ItemStack::new(ItemKind::OakPlanks, 1, None));
        }
        inventory.set_slot(
            PlayerInventory::SLOT_CRAFT_RESULT,
            ItemStack::new(ItemKind::CraftingTable, 1, None),
        );
    }
}
