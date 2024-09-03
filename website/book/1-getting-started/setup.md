# Before You Start

We recommend you get familiar with Bevy's Entity Component System architecture and API, as Valence uses the exact same crate for its ECS. You can find [Bevy's introduction to ECS here](https://bevyengine.org/learn/book/getting-started/ecs/).

You should also download Minecraft, obviously. You can use any launcher you'd like, but we recommend using the [Prism Launcher](https://prismlauncher.org/) as it will let you run offline mode clients a bit easier.

# Getting Started

The first thing you'll need to do is create a new binary Rust project. You can do this by running `cargo new --bin my_project` in your terminal. Once you've done that, you'll need to add Valence as a dependency:

```bash
cargo add valence
```

Next, you'll need to set up a new `App` in `main()`. This is the bare minimum you need to get a Valence app running.

```rust
use valence::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .run();
}
```

If you run this and try to join the server, you'll see "Joining world..." and then... nothing. It just sits there. That's because we need to tell the client what position to spawn at. Let's fix that.

# Hello World

Let's add a startup system that will put a single block under the spawn position. There's gonna be a lot of new stuff here, but don't worry, we'll briefly touch on most of it.

Chunk Layers are the way Valence handles worlds. A client can only view a single chunk layer at a time. So the first thing we need to do is create a new pair of chunk and entity layers (`LayerBundle`), add some chunks to it, and then set our desired block in the world.

```rust
fn setup(
    mut commands: Commands,
    server: Res<Server>,
    biomes: Res<BiomeRegistry>,
    dimensions: Res<DimensionTypeRegistry>,
) {
    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    // We have to add chunks to the world first, they start empty.
    for z in -5..5 {
        for x in -5..5 {
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    // This actually sets the block in the world.
    layer
        .chunk
        .set_block([0, 64, 0], BlockState::GRASS_BLOCK);

    // This spawns the layer into the world.
    commands.spawn(layer);
}
```

Now we need to handle clients when they join the server. Valence automatically spawns a new entity for each client that joins the server, we just need to add a system detects when clients are added.

```rust
fn init_clients(
    mut clients: Query<
        (
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
        pos.set([0.5, 65.0, 0.5]);
        *game_mode = GameMode::Creative;
    }
}
```

So what's going on here? Similar to chunk layers, there are also entity layers. However, unlike chunk layers, a client can view any number of entity layers. These can be used to show different entities to different clients.

So from top to bottom this code does the following:

1. Sets the client's _player entity_ to the layer we created in `setup()`. This makes the player visible to other clients viewing this layer.
2. Sets the client's visible chunk layer to the layer we created in `setup()`.
3. Sets the client's visible entity layers to include the layer we created in `setup()`.
4. Sets the client's position to the spawn position.
5. Sets the client's game mode to creative.

Finally, we'll need to add these systems to our `App`:

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, init_clients)
        .run();
}
```

That's it! You should now be able to run your server and join it from Minecraft. You should see a single grass block under your feet, and be able to move around and jump.
