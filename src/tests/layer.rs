use std::collections::BTreeSet;

use bevy_ecs::world::EntityMut;

use crate::client::{ViewDistance, VisibleEntityLayers};
use crate::entity::cow::CowEntityBundle;
use crate::entity::{EntityLayerId, Position};
use crate::layer::chunk::UnloadedChunk;
use crate::layer::{ChunkLayer, EntityLayer};
use crate::protocol::packets::play::{
    BlockEntityUpdateS2c, ChunkDataS2c, ChunkDeltaUpdateS2c, EntitiesDestroyS2c, EntitySpawnS2c,
    MoveRelativeS2c, UnloadChunkS2c,
};
use crate::protocol::Packet;
use crate::testing::ScenarioSingleClient;
use crate::{BlockState, ChunkView, Despawned, Server};

#[test]
fn block_create_destroy() {
    let ScenarioSingleClient {
        mut app,
        client: _,
        mut helper,
        layer: layer_ent,
    } = ScenarioSingleClient::new();

    let mut layer = app.world.get_mut::<ChunkLayer>(layer_ent).unwrap();

    // Insert an empty chunk at (0, 0).
    layer.insert_chunk([0, 0], UnloadedChunk::new());

    // Wait until the next tick to start sending changes.
    app.update();

    let mut layer = app.world.get_mut::<ChunkLayer>(layer_ent).unwrap();

    // Set some blocks.
    layer.set_block([1, 1, 1], BlockState::CHEST);
    layer.set_block([1, 2, 1], BlockState::PLAYER_HEAD);
    layer.set_block([1, 3, 1], BlockState::OAK_SIGN);

    app.update();

    {
        let recvd = helper.collect_received();

        recvd.assert_count::<ChunkDeltaUpdateS2c>(1);
        recvd.assert_count::<BlockEntityUpdateS2c>(3);
    }

    let mut layer = app.world.get_mut::<ChunkLayer>(layer_ent).unwrap();

    layer.set_block([1, 1, 1], BlockState::AIR);
    layer.set_block([1, 2, 1], BlockState::AIR);
    layer.set_block([1, 3, 1], BlockState::AIR);

    app.update();

    {
        let recvd = helper.collect_received();

        recvd.assert_count::<ChunkDeltaUpdateS2c>(1);
        recvd.assert_count::<BlockEntityUpdateS2c>(0);
    }
}

#[test]
fn layer_chunk_view_change() {
    fn view(client: &EntityMut) -> ChunkView {
        let chunk_pos = client.get::<Position>().unwrap().0.into();
        let view_dist = client.get::<ViewDistance>().unwrap().get();

        ChunkView::new(chunk_pos, view_dist)
    }

    let ScenarioSingleClient {
        mut app,
        client: client_ent,
        mut helper,
        layer: layer_ent,
    } = ScenarioSingleClient::new();

    let mut layer = app.world.get_mut::<ChunkLayer>(layer_ent).unwrap();

    for z in -30..30 {
        for x in -30..30 {
            layer.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    let mut client = app.world.entity_mut(client_ent);

    client.get_mut::<Position>().unwrap().set([8.0, 0.0, 8.0]);
    client.get_mut::<ViewDistance>().unwrap().set(6);

    // Tick
    app.update();
    let mut client = app.world.entity_mut(client_ent);

    let mut loaded_chunks = BTreeSet::new();

    // Collect all chunks received on join.
    for f in helper.collect_received().0 {
        if f.id == ChunkDataS2c::ID {
            let ChunkDataS2c { pos, .. } = f.decode::<ChunkDataS2c>().unwrap();
            // Newly received chunk was not previously loaded.
            assert!(loaded_chunks.insert(pos), "({pos:?})");
        }
    }

    // Check that all the received chunks are in the client's view.
    for pos in view(&client).iter() {
        assert!(loaded_chunks.contains(&pos), "{pos:?}");
    }

    assert!(!loaded_chunks.is_empty());

    // Move the client to the adjacent chunk.
    client.get_mut::<Position>().unwrap().set([24.0, 0.0, 24.0]);

    // Tick
    app.update();
    let client = app.world.entity_mut(client_ent);

    // For all chunks received this tick...
    for f in helper.collect_received().0 {
        match f.id {
            ChunkDataS2c::ID => {
                let ChunkDataS2c { pos, .. } = f.decode().unwrap();
                // Newly received chunk was not previously loaded.
                assert!(loaded_chunks.insert(pos), "({pos:?})");
            }
            UnloadChunkS2c::ID => {
                let UnloadChunkS2c { pos } = f.decode().unwrap();
                // Newly removed chunk was previously loaded.
                assert!(loaded_chunks.remove(&pos), "({pos:?})");
            }
            _ => {}
        }
    }

    // Check that all chunks loaded now are within the client's view.
    for pos in view(&client).iter() {
        assert!(loaded_chunks.contains(&pos), "{pos:?}");
    }
}

#[test]
fn chunk_viewer_count() {
    let ScenarioSingleClient {
        mut app,
        client: client_ent,
        mut helper,
        layer: layer_ent,
    } = ScenarioSingleClient::new();

    let mut client = app.world.entity_mut(client_ent);

    client.get_mut::<Position>().unwrap().set([8.0, 64.0, 8.0]);
    client.get_mut::<ViewDistance>().unwrap().set(2);

    let mut layer = app.world.get_mut::<ChunkLayer>(layer_ent).unwrap();

    // Create chunk at (0, 0).
    layer.insert_chunk([0, 0], UnloadedChunk::new());

    app.update(); // Tick.

    helper.collect_received().assert_count::<ChunkDataS2c>(1);

    let mut layer = app.world.get_mut::<ChunkLayer>(layer_ent).unwrap();

    assert_eq!(layer.chunk_mut([0, 0]).unwrap().viewer_count(), 1);

    // Create new chunk next to the first chunk and move the client away from it on
    // the same tick.
    layer.insert_chunk([0, 1], UnloadedChunk::new());

    let mut client = app.world.entity_mut(client_ent);
    client.get_mut::<Position>().unwrap().set([100.0, 0.0, 0.0]);

    app.update(); // Tick.

    {
        let recvd = helper.collect_received();

        recvd.assert_count::<ChunkDataS2c>(1);
        recvd.assert_count::<UnloadChunkS2c>(2);
    }

    let mut layer = app.world.get_mut::<ChunkLayer>(layer_ent).unwrap();

    // Viewer count of both chunks should be zero.
    assert_eq!(layer.chunk([0, 0]).unwrap().viewer_count(), 0);
    assert_eq!(layer.chunk([0, 1]).unwrap().viewer_count(), 0);

    // Create a third chunk adjacent to the others.
    layer.insert_chunk([1, 0], UnloadedChunk::new());

    // Move the client back in view of all three chunks.
    let mut client = app.world.entity_mut(client_ent);
    client.get_mut::<Position>().unwrap().set([8.0, 0.0, 8.0]);

    app.update(); // Tick.

    let mut layer = app.world.get_mut::<ChunkLayer>(layer_ent).unwrap();

    // All three chunks should have viewer count of one.
    assert_eq!(layer.chunk_mut([0, 0]).unwrap().viewer_count(), 1);
    assert_eq!(layer.chunk_mut([1, 0]).unwrap().viewer_count(), 1);
    assert_eq!(layer.chunk_mut([0, 1]).unwrap().viewer_count(), 1);

    // Client should have received load packet for all three.
    helper.collect_received().assert_count::<ChunkDataS2c>(3);
}

#[test]
fn entity_layer_switching() {
    let ScenarioSingleClient {
        mut app,
        client: client_ent,
        mut helper,
        layer: l1,
    } = ScenarioSingleClient::new();

    let server = app.world.resource::<Server>();

    let l2 = EntityLayer::new(server);
    let l3 = EntityLayer::new(server);

    let l2 = app.world.spawn(l2).id();
    let _l3 = app.world.spawn(l3).id();

    // Spawn three entities and put them all on the main layer to start.

    let e1 = CowEntityBundle {
        layer: EntityLayerId(l1),
        ..Default::default()
    };

    let e2 = CowEntityBundle {
        layer: EntityLayerId(l1),
        ..Default::default()
    };

    let e3 = CowEntityBundle {
        layer: EntityLayerId(l1),
        ..Default::default()
    };

    let e1 = app.world.spawn(e1).id();
    let _e2 = app.world.spawn(e2).id();
    let _e3 = app.world.spawn(e3).id();

    app.update(); // Tick.

    // Can the client see all the new entities?
    helper.collect_received().assert_count::<EntitySpawnS2c>(3);

    // Move e1 to l2 and add l2 to the visible layers set.
    app.world.get_mut::<EntityLayerId>(e1).unwrap().0 = l2;
    app.world
        .get_mut::<VisibleEntityLayers>(client_ent)
        .unwrap()
        .0
        .insert(l2);

    app.update(); // Tick.

    {
        let recvd = helper.collect_received();

        // Client received packets to despawn and then spawn the entity in the new
        // layer. (this could be optimized away in the future)
        recvd.assert_count::<EntitiesDestroyS2c>(1);
        recvd.assert_count::<EntitySpawnS2c>(1);
        recvd.assert_order::<(EntitiesDestroyS2c, EntitySpawnS2c)>();
    }

    // Remove the original layer from the visible layer set.
    assert!(app
        .world
        .get_mut::<VisibleEntityLayers>(client_ent)
        .unwrap()
        .0
        .remove(&l1));

    app.update(); // Tick.

    // Both entities on the original layer should be removed.
    {
        let recvd = helper.collect_received();
        recvd.assert_count::<EntitiesDestroyS2c>(1);
    }

    // Despawn l2.
    app.world.entity_mut(l2).insert(Despawned);

    app.update(); // Tick.

    // e1 should be removed.
    helper
        .collect_received()
        .assert_count::<EntitiesDestroyS2c>(1);

    let mut visible_entity_layers = app
        .world
        .get_mut::<VisibleEntityLayers>(client_ent)
        .unwrap();

    // l2 should be automatically removed from the visible layers set.
    assert!(!visible_entity_layers.0.contains(&e1));

    // Add back the original layer.
    assert!(visible_entity_layers.0.insert(l1));

    app.update(); // Tick.

    // e2 and e3 should be spawned.

    {
        let recvd = helper.collect_received();

        recvd.assert_count::<EntitySpawnS2c>(2);
    }
}

#[test]
fn chunk_entity_spawn_despawn() {
    let ScenarioSingleClient {
        mut app,
        client: client_ent,
        mut helper,
        layer: layer_ent,
    } = ScenarioSingleClient::new();

    let mut layer = app.world.get_mut::<ChunkLayer>(layer_ent).unwrap();

    // Insert an empty chunk at (0, 0).
    layer.insert_chunk([0, 0], UnloadedChunk::new());

    // Put an entity in the new chunk.
    let cow_ent = app
        .world
        .spawn(CowEntityBundle {
            position: Position::new([8.0, 0.0, 8.0]),
            layer: EntityLayerId(layer_ent),
            ..Default::default()
        })
        .id();

    app.update();

    // Client is in view of the chunk, so they should receive exactly one chunk
    // spawn packet and entity spawn packet.
    {
        let recvd = helper.collect_received();

        recvd.assert_count::<ChunkDataS2c>(1);
        recvd.assert_count::<EntitySpawnS2c>(1);
        recvd.assert_count::<UnloadChunkS2c>(0);
        recvd.assert_count::<EntitiesDestroyS2c>(0);
    }

    // Move the entity. Client should receive entity move packet.
    app.world.get_mut::<Position>(cow_ent).unwrap().0.x += 0.1;

    app.update();

    helper.collect_received().assert_count::<MoveRelativeS2c>(1);

    // Despawning the chunk should delete the chunk and not the entity contained
    // within.
    let mut layer = app.world.get_mut::<ChunkLayer>(layer_ent).unwrap();

    layer.remove_chunk([0, 0]).unwrap();

    app.update();

    {
        let recvd = helper.collect_received();

        recvd.assert_count::<UnloadChunkS2c>(1);
        recvd.assert_count::<EntitiesDestroyS2c>(0);
        recvd.assert_count::<ChunkDataS2c>(0);
        recvd.assert_count::<EntitySpawnS2c>(0);
    }

    // Placing the chunk back should respawn the chunk and not the entity.

    let mut layer = app.world.get_mut::<ChunkLayer>(layer_ent).unwrap();

    assert!(layer.insert_chunk([0, 0], UnloadedChunk::new()).is_none());

    app.update();

    {
        let recvd = helper.collect_received();

        recvd.assert_count::<ChunkDataS2c>(1);
        recvd.assert_count::<EntitySpawnS2c>(0);
        recvd.assert_count::<UnloadChunkS2c>(0);
        recvd.assert_count::<EntitiesDestroyS2c>(0);
    }

    // Move player and entity away from the chunk on the same tick.

    app.world.get_mut::<Position>(client_ent).unwrap().0.x = 1000.0;
    app.world.get_mut::<Position>(cow_ent).unwrap().0.x = -1000.0;

    app.update();

    {
        let recvd = helper.collect_received();

        recvd.assert_count::<UnloadChunkS2c>(1);
        recvd.assert_count::<EntitiesDestroyS2c>(1);
        recvd.assert_count::<ChunkDataS2c>(0);
        recvd.assert_count::<EntitySpawnS2c>(0);
    }

    // Put the client and entity back on the same tick.

    app.world
        .get_mut::<Position>(client_ent)
        .unwrap()
        .set([8.0, 0.0, 8.0]);
    app.world
        .get_mut::<Position>(cow_ent)
        .unwrap()
        .set([8.0, 0.0, 8.0]);

    app.update();

    {
        let recvd = helper.collect_received();

        recvd.assert_count::<ChunkDataS2c>(1);
        recvd.assert_count::<EntitySpawnS2c>(1);
        recvd.assert_count::<UnloadChunkS2c>(0);
        recvd.assert_count::<EntitiesDestroyS2c>(0);
    }

    // Adding and removing a chunk on the same tick should have no effect on
    // the client.

    let mut layer = app.world.get_mut::<ChunkLayer>(layer_ent).unwrap();

    layer.insert_chunk([0, 1], UnloadedChunk::new());
    layer.remove_chunk([0, 1]).unwrap();

    app.world
        .get_mut::<Position>(cow_ent)
        .unwrap()
        .set([24.0, 0.0, 24.0]);

    app.update();

    {
        let recvd = helper.collect_received();

        recvd.assert_count::<ChunkDataS2c>(0);
        recvd.assert_count::<EntitySpawnS2c>(0);
        recvd.assert_count::<UnloadChunkS2c>(0);
        recvd.assert_count::<EntitiesDestroyS2c>(0);
    }
}
