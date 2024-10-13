use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_equipment::Equipment;
use valence_server::entity::zombie::ZombieEntityBundle;
use valence_server::protocol::packets::play::EntityEquipmentUpdateS2c;
use valence_server::{ItemKind, ItemStack};

use crate::testing::ScenarioSingleClient;

#[test]
fn test_only_send_update_to_other_players() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        ..
    } = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    let mut player_equipment = app
        .world_mut()
        .get_mut::<Equipment>(client)
        .expect("could not get player equipment");

    player_equipment.set_chestplate(ItemStack::new(ItemKind::DiamondChestplate, 1, None));

    app.update();

    // Make assertions
    let sent_packets = helper.collect_received();

    // We only have one player, so we should not have sent any packets.
    sent_packets.assert_count::<EntityEquipmentUpdateS2c>(0);
}

#[test]
fn test_multiple_entities() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        ..
    } = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    let zombie_bundle = ZombieEntityBundle {
        ..Default::default()
    };

    let zombie = app.world_mut().spawn(zombie_bundle).id();

    // add equipment to the zombie,its not attached to the zombie by default
    app.world_mut()
        .commands()
        .entity(zombie)
        .insert(Equipment::default());

    app.update();

    let mut equipment = app
        .world_mut()
        .get_mut::<Equipment>(zombie)
        .expect("could not get entity equipment");

    equipment.set_chestplate(ItemStack::new(ItemKind::DiamondChestplate, 1, None));
    equipment.set_helmet(ItemStack::new(ItemKind::DiamondHelmet, 1, None));

    app.update();

    // Make assertions
    let sent_packets = helper.collect_received();
    sent_packets.assert_count::<EntityEquipmentUpdateS2c>(1);

    helper.clear_received();

    let mut equipment = app
        .world_mut()
        .get_mut::<Equipment>(zombie)
        .expect("could not get entity equipment");

    // Set the zombie's equipment to the same items
    equipment.set_chestplate(ItemStack::new(ItemKind::DiamondChestplate, 1, None));
    equipment.set_helmet(ItemStack::new(ItemKind::DiamondHelmet, 1, None));

    app.update();

    // Make assertions
    let sent_packets = helper.collect_received();
    sent_packets.assert_count::<EntityEquipmentUpdateS2c>(0);
}
