use valence_server::entity::living::Health;
use valence_server::entity::player::{Food, Saturation};
use valence_server::protocol::VarInt;
use valence_server::protocol::packets::play::HealthUpdateS2c;

use crate::testing::ScenarioSingleClient;

#[test]
fn test_hunger() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    app.update();
    helper.clear_received();

    let og_saturation = app.world.get::<Saturation>(client).unwrap().0;
    let og_health = app.world.get::<Health>(client).unwrap().0;

    // set food level to 0
    app.world.get_mut::<Food>(client).unwrap().0 = 0;

    app.update();

    // make sure the packet was sent
    let sent_packets = helper.collect_received();
    
    sent_packets.assert_count::<HealthUpdateS2c>(1);

    let packet = sent_packets.first::<HealthUpdateS2c>();

    assert_eq!(packet.health, og_health);
    assert_eq!(packet.food, VarInt(0));
    assert_eq!(packet.food_saturation, og_saturation);
}
