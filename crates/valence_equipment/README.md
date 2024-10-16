# `valence_equipment`
Manages Minecraft's entity equipment (armor, held items) via the `Equipment` component.
By default this is separated from an entities `Inventory` (which means that changes are only visible to other players), but it can be synced by attaching the `EquipmentInventorySync`
component to a entity (currently only Players).
