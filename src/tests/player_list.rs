use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_client::packet::PlayerSpawnS2c;
use valence_instance::chunk::UnloadedChunk;
use valence_instance::Instance;
use valence_player_list::packet::PlayerListS2c;

use super::{create_mock_client, scenario_single_client};

#[test]
fn player_list_arrives_before_player_spawn() {
    let mut app = App::new();

    let (_client_ent_1, mut client_helper_1) = scenario_single_client(&mut app);

    let (inst_ent, mut inst) = app
        .world
        .query::<(Entity, &mut Instance)>()
        .get_single_mut(&mut app.world)
        .unwrap();

    for z in -5..5 {
        for x in -5..5 {
            inst.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    app.update();

    {
        let recvd = client_helper_1.collect_received();
        recvd.assert_count::<PlayerListS2c>(1);
        recvd.assert_count::<PlayerSpawnS2c>(0);
        recvd.assert_order::<(PlayerListS2c, PlayerSpawnS2c)>();

        let pkt = recvd.first::<PlayerListS2c>();
        assert!(pkt.actions.add_player());
        assert_eq!(pkt.entries.len(), 1);
    }

    let (mut client_2, mut client_helper_2) = create_mock_client("test_2");
    client_2.player.location.0 = inst_ent;

    app.world.spawn(client_2);

    app.update();

    {
        let recvd = client_helper_1.collect_received();
        recvd.assert_count::<PlayerListS2c>(1);
        recvd.assert_count::<PlayerSpawnS2c>(1);
        recvd.assert_order::<(PlayerListS2c, PlayerSpawnS2c)>();

        let pkt = recvd.first::<PlayerListS2c>();
        assert!(pkt.actions.add_player());
        assert_eq!(pkt.entries.len(), 1);
    }

    {
        let recvd = client_helper_2.collect_received();
        recvd.assert_count::<PlayerListS2c>(1);
        recvd.assert_count::<PlayerSpawnS2c>(1);
        recvd.assert_order::<(PlayerListS2c, PlayerSpawnS2c)>();

        let pkt = recvd.first::<PlayerListS2c>();
        assert!(pkt.actions.add_player());
        assert_eq!(pkt.entries.len(), 2);
    }

    /*
    {
        let recvd = client_helper_1.collect_received();
        recvd.assert_count::<PlayerListS2c>(1);
        recvd.assert_count::<PlayerSpawnS2c>(1);
        recvd.assert_order::<(PlayerListS2c, PlayerSpawnS2c)>();

        let pkt = recvd.first::<PlayerListS2c>();
        assert!(pkt.actions.add_player());
        assert_eq!(pkt.entries.len(), 2);
    }

    {
        let recvd = client_helper_2.collect_received();
        recvd.assert_count::<PlayerListS2c>(1);
        recvd.assert_count::<PlayerSpawnS2c>(1);
        recvd.assert_order::<(PlayerListS2c, PlayerSpawnS2c)>();

        let pkt = recvd.first::<PlayerListS2c>();
        assert!(pkt.actions.add_player());
        assert_eq!(pkt.entries.len(), 2);
    }*/
}
