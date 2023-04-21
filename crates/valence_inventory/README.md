# valence_inventory

The inventory system.

This module contains the systems and components needed to handle
inventories. By default, clients will have a player inventory attached to
them.

# Components

- [`Inventory`]: The inventory component. This is the thing that holds
  items.
- [`OpenInventory`]: The component that is attached to clients when they
  have an inventory open.

# Examples

An example system that will let you access all player's inventories:

```
# use bevy_ecs::prelude::*;
# use valence_inventory::*;
# use valence_client::Client;
fn system(clients: Query<(&Client, &Inventory)>) {}
```

### See also

Examples related to inventories in the `valence/examples/` directory:
- `building`
- `chest`
