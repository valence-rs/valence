use std::collections::BTreeSet;

use bevy_app::App;
use bevy_ecs::world::EntityMut;
use valence_client::ViewDistance;
use valence_core::chunk_pos::ChunkView;
use valence_entity::Position;
use valence_instance::packet::{ChunkDataS2c, UnloadChunkS2c};
use valence_instance::Chunk;

use super::*;

#[test]
fn client_chunk_view_change() {
    let mut app = App::new();

    let (client_ent, mut client_helper) = scenario_single_client(&mut app);

    let mut instance = app
        .world
        .query::<&mut Instance>()
        .single_mut(&mut app.world);

    for z in -15..15 {
        for x in -15..15 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    let mut client = app.world.entity_mut(client_ent);

    client.get_mut::<Position>().unwrap().set([8.0, 0.0, 8.0]);
    client.get_mut::<ViewDistance>().unwrap().set(6);

    // Tick
    app.update();
    let mut client = app.world.entity_mut(client_ent);

    let mut loaded_chunks = BTreeSet::new();

    for f in client_helper.collect_sent().0 {
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

    for f in client_helper.collect_sent().0 {
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

fn view(client: &EntityMut) -> ChunkView {
    let chunk_pos = client.get::<Position>().unwrap().chunk_pos();
    let view_dist = client.get::<ViewDistance>().unwrap().get();

    ChunkView::new(chunk_pos, view_dist)
}
