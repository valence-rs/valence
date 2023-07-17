use valence_block::BlockState;
use valence_layer::chunk::UnloadedChunk;
use valence_layer::packet::{BlockEntityUpdateS2c, ChunkDeltaUpdateS2c};
use valence_layer::ChunkLayer;

use crate::testing::ScenarioSingleClient;

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
