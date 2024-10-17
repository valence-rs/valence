# `valence_equipment`
Manages Minecraft's entity equipment (armor, held items) via the `Equipment` component.
By default this is separated from an entities `Inventory` (which means that changes are only visible to other players), but it can be synced by attaching the `EquipmentInventorySync`
component to a entity (currently only Players).

## Example

```rust 
use valence::prelude::*;
use valence_equipment::*;

// Spawn a player with full armor, a sword and a shield.
fn init_clients(
    mut commands: Commands,
    mut clients: Query<
        (
            Entity,
            &mut Position,
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for (
        player,
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

        let equipment = Equipment::new(
            ItemStack::new(ItemKind::DiamondSword, 1, None), // Main hand
            ItemStack::new(ItemKind::Shield, 1, None), // Off hand
            ItemStack::new(ItemKind::DiamondBoots, 1, None), // Feet
            ItemStack::new(ItemKind::DiamondLeggings, 1, None), // Legs
            ItemStack::new(ItemKind::DiamondChestplate, 1, None), // Chest
            ItemStack::new(ItemKind::DiamondHelmet, 1, None), // Head
        );

        commands.entity(player).insert(equipment); // Add the equipment to the player

        commands.entity(player).insert(EquipmentInventorySync); // Sync the equipment with the player's inventory. This is not required if you want to only show equipment for other players.
    }
}
```

