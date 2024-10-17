# `valence_equipment`
Manages Minecraft's entity equipment (armor, held items) via the `Equipment` component.
By default this is separated from an entities `Inventory` (which means that changes are only visible to other players), but it can be synced by attaching the `EquipmentInventorySync`
component to a entity (currently only Players).

## Example

```rust 
use bevy_ecs::prelude::*;
use valence_equipment::*;
use valence_server::{
    ItemStack, ItemKind,
    entity::player::PlayerEntity,
};
// Add equipment to players when they are added to the world.
fn init_equipment(
    mut clients: Query<
        &mut Equipment,
        (
            Added<Equipment>,
            With<PlayerEntity>,
        ),
    >,
) {
    for mut equipment in &mut clients
    {
        equipment.set_main_hand(ItemStack::new(ItemKind::DiamondSword, 1, None));
        equipment.set_off_hand(ItemStack::new(ItemKind::Shield, 1, None));
        equipment.set_feet(ItemStack::new(ItemKind::DiamondBoots, 1, None));
        equipment.set_legs(ItemStack::new(ItemKind::DiamondLeggings, 1, None));
        equipment.set_chest(ItemStack::new(ItemKind::DiamondChestplate, 1, None));
        equipment.set_head(ItemStack::new(ItemKind::DiamondHelmet, 1, None));
    }
}
```

### See also

Examples related to inventories in the `valence/examples/` directory:
- `equipment`

