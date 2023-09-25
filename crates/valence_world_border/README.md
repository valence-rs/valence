# valence_world_border

Contains the plugin for working with Minecraft's [world border](https://minecraft.fandom.com/wiki/World_border).

To enable world border functionality for a layer, insert the [`WorldBorderBundle`] component on the layer entity.
Note that the layer entity must have the [`ChunkLayer`] component for this to work.

## Example

```rust
use bevy_ecs::prelude::*;
use valence_world_border::*;

fn example_system(mut world_borders: Query<(&mut WorldBorderCenter, &mut WorldBorderLerp)>) {
    for (mut center, mut lerp) in &mut world_borders {
        // Change the center position of the world border.
        center.x = 123.0;
        center.z = 456.0;

        // Change the diameter of the world border.
        // If you want to change the diameter without interpolating, stop after this.
        lerp.target_diameter = 100.0;

        // Have the world border linearly interpolate its diameter from 50 to 100 over 200 ticks.
        // `current_diameter` and `remaining_ticks` will change automatically, but you can modify their values at any time.
        lerp.current_diameter = 50.0;
        lerp.remaining_ticks = 200;
    }
}
