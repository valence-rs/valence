#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use bevy_app::App;
    use bevy_ecs::world::EntityMut;
    use valence_core::packet::s2c::play::ChunkDataS2c;
    use valence_core::packet::S2cPlayPacket;
    use valence_instance::Chunk;

    use super::*;
    use crate::unit_test::util::scenario_single_client;

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
        client.get_mut::<ViewDistance>().unwrap().0 = 6;

        // Tick
        app.update();
        let mut client = app.world.entity_mut(client_ent);

        let mut loaded_chunks = BTreeSet::new();

        for pkt in client_helper.collect_sent() {
            if let S2cPlayPacket::ChunkDataS2c(ChunkDataS2c { pos, .. }) = pkt {
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

        for pkt in client_helper.collect_sent() {
            match pkt {
                S2cPlayPacket::ChunkDataS2c(ChunkDataS2c { pos, .. }) => {
                    assert!(loaded_chunks.insert(pos), "({pos:?})");
                }
                S2cPlayPacket::UnloadChunkS2c(UnloadChunkS2c { pos }) => {
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
        let view_dist = client.get::<ViewDistance>().unwrap().0;

        ChunkView::new(chunk_pos, view_dist)
    }
}