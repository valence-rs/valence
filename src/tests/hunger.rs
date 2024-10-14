use valence_server::entity::living::Health;
use valence_server::entity::player::{Food, Saturation};
use valence_server::protocol::packets::play::SetHealthS2c;
use valence_server::protocol::VarInt;

use crate::testing::ScenarioSingleClient;

#[test]
fn test_hunger() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        ..
    } = ScenarioSingleClient::new();

    app.update();
    helper.clear_received();

    let og_saturation = app.world_mut().get::<Saturation>(client).unwrap().0;
    let og_health = app.world_mut().get::<Health>(client).unwrap().0;

    // set food level to 5
    app.world_mut().get_mut::<Food>(client).unwrap().0 = 5;

    app.update();

    // make sure the packet was sent
    let sent_packets = helper.collect_received();

    sent_packets.assert_count::<SetHealthS2c>(1);

    let packet = sent_packets.first::<SetHealthS2c>();

    assert_eq!(packet.health, og_health);
    assert_eq!(packet.food, VarInt(5));
    assert_eq!(packet.food_saturation, og_saturation);
}
