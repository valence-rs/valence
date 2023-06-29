use std::collections::BTreeSet;

use bevy_app::App;
use bevy_ecs::world::EntityMut;
use valence_client::{Client, ViewDistance};
use valence_core::chunk_pos::ChunkView;
use valence_entity::cow::CowEntityBundle;
use valence_entity::packet::{EntitiesDestroyS2c, EntitySpawnS2c, MoveRelativeS2c};
use valence_entity::Position;
use valence_instance::chunk::UnloadedChunk;
use valence_instance::packet::{ChunkDataS2c, UnloadChunkS2c};

use super::*;

#[test]
fn client_chunk_view_change() {
    fn view(client: &EntityMut) -> ChunkView {
        let chunk_pos = client.get::<Position>().unwrap().chunk_pos();
        let view_dist = client.get::<ViewDistance>().unwrap().get();

        ChunkView::new(chunk_pos, view_dist)
    }

    let mut app = App::new();

    let (client_ent, mut client_helper) = scenario_single_client(&mut app);

    let mut instance = app
        .world
        .query::<&mut Instance>()
        .single_mut(&mut app.world);

    for z in -15..15 {
        for x in -15..15 {
            instance.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    let mut client = app.world.entity_mut(client_ent);

    client.get_mut::<Position>().unwrap().set([8.0, 0.0, 8.0]);
    client.get_mut::<ViewDistance>().unwrap().set(6);

    // Tick
    app.update();
    let mut client = app.world.entity_mut(client_ent);

    let mut loaded_chunks = BTreeSet::new();

    for f in client_helper.collect_received().0 {
        if f.id == ChunkDataS2c::ID {
            let ChunkDataS2c { pos, .. } = f.decode::<ChunkDataS2c>().unwrap();
            assert!(loaded_chunks.insert(pos), "({pos:?})");
        }
    }

    for pos in view(&client).iter() {
        assert!(loaded_chunks.contains(&pos), "{pos:?}");
    }

    assert!(!loaded_chunks.is_empty());

    // Move the client to the adjacent chunk.
    client.get_mut::<Position>().unwrap().set([24.0, 0.0, 24.0]);

    // Tick
    app.update();
    let client = app.world.entity_mut(client_ent);

    for f in client_helper.collect_received().0 {
        match f.id {
            ChunkDataS2c::ID => {
                let ChunkDataS2c { pos, .. } = f.decode().unwrap();
                assert!(loaded_chunks.insert(pos), "({pos:?})");
            }
            UnloadChunkS2c::ID => {
                let UnloadChunkS2c { pos } = f.decode().unwrap();
                assert!(loaded_chunks.remove(&pos), "({pos:?})");
            }
            _ => {}
        }
    }

    for pos in view(&client).iter() {
        assert!(loaded_chunks.contains(&pos), "{pos:?}");
    }
}

#[test]
fn entity_chunk_spawn_despawn() {
    let mut app = App::new();

    let (client_ent, mut client_helper) = scenario_single_client(&mut app);

    let (inst_ent, mut inst) = app
        .world
        .query::<(Entity, &mut Instance)>()
        .single_mut(&mut app.world);

    // Insert an empty chunk at (0, 0).
    inst.insert_chunk([0, 0], UnloadedChunk::new());

    // Put an entity in the new chunk.
    let cow_ent = app
        .world
        .spawn(CowEntityBundle {
            position: Position::new([8.0, 0.0, 8.0]),
            location: Location(inst_ent),
            ..Default::default()
        })
        .id();

    app.update();

    // Client is in view of the chunk, so they should receive exactly one chunk
    // spawn packet and entity spawn packet.
    let recvd = client_helper.collect_received();

    recvd.assert_count::<ChunkDataS2c>(1);
    recvd.assert_count::<EntitySpawnS2c>(1);
    recvd.assert_count::<UnloadChunkS2c>(0);
    recvd.assert_count::<EntitiesDestroyS2c>(0);

    // Move the entity. Client should receive entity move packet.
    app.world.get_mut::<Position>(cow_ent).unwrap().0.x += 0.1;

    app.update();

    client_helper
        .collect_received()
        .assert_count::<MoveRelativeS2c>(1);

    // Despawning the chunk should delete the chunk and the entity contained within.
    let mut inst = app.world.get_mut::<Instance>(inst_ent).unwrap();

    inst.remove_chunk([0, 0]).unwrap();

    app.update();

    let recvd = client_helper.collect_received();

    recvd.assert_count::<UnloadChunkS2c>(1);
    recvd.assert_count::<EntitiesDestroyS2c>(1);
    recvd.assert_count::<ChunkDataS2c>(0);
    recvd.assert_count::<EntitySpawnS2c>(0);

    // Placing the chunk back should respawn the orphaned entity.

    let mut inst = app.world.get_mut::<Instance>(inst_ent).unwrap();

    assert!(inst.insert_chunk([0, 0], UnloadedChunk::new()).is_none());

    app.update();

    let recvd = client_helper.collect_received();

    recvd.assert_count::<ChunkDataS2c>(1);
    recvd.assert_count::<EntitySpawnS2c>(1);
    recvd.assert_count::<UnloadChunkS2c>(0);
    recvd.assert_count::<EntitiesDestroyS2c>(0);

    // TODO: add case for spawning chunk and moving entity into it at the same
    // time. TODO: adding and removing chunk on same tick should not send
    // any packets.
}
