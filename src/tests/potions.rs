use valence_server::{
    entity::active_status_effects::{ActiveStatusEffect, ActiveStatusEffects},
    protocol::{
        packets::play::{EntityStatusEffectS2c, RemoveEntityStatusEffectS2c},
        status_effects::StatusEffect,
        VarInt,
    },
};

use crate::testing::ScenarioSingleClient;

#[test]
fn test_status_effects_packets() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    // Add a potion effect to the client.
    let mut effects = app
        .world
        .get_mut::<ActiveStatusEffects>(client)
        .expect("Client should have status effects");
    effects.apply(
        ActiveStatusEffect::from_effect(StatusEffect::BadOmen)
            .with_duration(100)
            .with_amplifier(1),
    );

    // Update the server.
    app.update();

    // Make assertions
    let sent_packets = helper.collect_received();

    sent_packets.assert_count::<EntityStatusEffectS2c>(1);

    let packet = sent_packets.first::<EntityStatusEffectS2c>();

    assert_eq!(packet.entity_id, VarInt(0)); // Client entity ID is always 0
    assert_eq!(packet.effect_id, VarInt(31)); // Bad Omen
    assert_eq!(packet.amplifier, 1);
    assert_eq!(packet.duration, VarInt(100));

    // Clear the potion effect
    for _ in 0..99 {
        app.update();
    }

    helper.clear_received();
    app.update();

    // Make assertions
    let effects = app
        .world
        .get::<ActiveStatusEffects>(client)
        .expect("Client should have status effects");

    assert_eq!(effects.get_current_effect(StatusEffect::BadOmen), None);

    let sent_packets = helper.collect_received();

    sent_packets.assert_count::<RemoveEntityStatusEffectS2c>(1);

    let packet = sent_packets.first::<RemoveEntityStatusEffectS2c>();

    assert_eq!(packet.entity_id, VarInt(0)); // Client entity ID is always 0
    assert_eq!(packet.effect_id, VarInt(31)); // Bad Omen
}
