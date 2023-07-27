use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_block::BlockState;
use valence_instance::chunk::UnloadedChunk;
use valence_instance::packet::{BlockEntityUpdateS2c, ChunkDeltaUpdateS2c};
use valence_instance::Instance;

use crate::testing::scenario_single_client;

#[test]
fn block_create_destroy() {
    let mut app = App::new();

    let (_client_ent, mut client_helper) = scenario_single_client(&mut app);

    let (inst_ent, mut inst) = app
        .world
        .query::<(Entity, &mut Instance)>()
        .single_mut(&mut app.world);

    // Insert an empty chunk at (0, 0).
    inst.insert_chunk([0, 0], UnloadedChunk::new());

    // Wait until the next tick to start sending changes.
    app.update();

    let mut inst = app.world.get_mut::<Instance>(inst_ent).unwrap();

    // Set some blocks.
    inst.set_block([1, 1, 1], BlockState::CHEST);
    inst.set_block([1, 2, 1], BlockState::PLAYER_HEAD);
    inst.set_block([1, 3, 1], BlockState::OAK_SIGN);

    app.update();

    {
        let recvd = client_helper.collect_received();

        recvd.assert_count::<ChunkDeltaUpdateS2c>(1);
        recvd.assert_count::<BlockEntityUpdateS2c>(3);
    }

    let mut inst = app.world.get_mut::<Instance>(inst_ent).unwrap();

    inst.set_block([1, 1, 1], BlockState::AIR);
    inst.set_block([1, 2, 1], BlockState::AIR);
    inst.set_block([1, 3, 1], BlockState::AIR);

    app.update();

    {
        let recvd = client_helper.collect_received();

        recvd.assert_count::<ChunkDeltaUpdateS2c>(1);
        recvd.assert_count::<BlockEntityUpdateS2c>(0);
    }
}
