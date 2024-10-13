use valence_equipment::Equipment;
use valence_server::entity::armor_stand::ArmorStandEntityBundle;
use valence_server::entity::item::ItemEntityBundle;
use valence_server::entity::zombie::ZombieEntityBundle;
use valence_server::entity::{EntityLayerId, Position};
use valence_server::math::DVec3;
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
        mut helper,
        layer,
        ..
    } = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    let zombie_bundle = ZombieEntityBundle {
        layer: EntityLayerId(layer),
        ..Default::default()
    };

    let zombie = app.world_mut().spawn(zombie_bundle).id();

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

#[test]
fn test_update_on_load_entity() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer,
    } = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    let zombie_bundle = ZombieEntityBundle {
        layer: EntityLayerId(layer),
        position: Position::new(DVec3::new(1000.0, 0.0, 1000.0)),
        ..Default::default()
    };

    let zombie = app
        .world_mut()
        .spawn(zombie_bundle)
        .insert(Equipment::default())
        .id();

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
    // The zombie is not in range of the player
    sent_packets.assert_count::<EntityEquipmentUpdateS2c>(0);

    // Move the player to the zombie
    let mut player_pos = app
        .world_mut()
        .get_mut::<Position>(client)
        .expect("could not get player position");

    player_pos.0 = DVec3::new(1000.0, 0.0, 1000.0);

    // 1 tick for the tp, 1 tick for loading the entity (i think)
    app.update();
    app.update();

    // Make assertions
    let sent_packets = helper.collect_received();
    // Once the player is in range, we send the equipment update
    sent_packets.assert_count::<EntityEquipmentUpdateS2c>(1);
}

#[test]
fn test_skip_update_for_empty_equipment() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer,
    } = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    let zombie_bundle = ZombieEntityBundle {
        layer: EntityLayerId(layer),
        position: Position::new(DVec3::new(1000.0, 0.0, 1000.0)),
        ..Default::default()
    };

    app.world_mut()
        .spawn(zombie_bundle)
        .insert(Equipment::default());

    app.update();
    app.update();

    // Make assertions
    let sent_packets = helper.collect_received();
    // The zombie is not in range of the player
    sent_packets.assert_count::<EntityEquipmentUpdateS2c>(0);

    // Move the player to the zombie
    let mut player_pos = app
        .world_mut()
        .get_mut::<Position>(client)
        .expect("could not get player position");

    player_pos.0 = DVec3::new(1000.0, 0.0, 1000.0);

    // 1 tick for the tp, 1 tick for loading the entity (i think)
    app.update();
    app.update();

    // Make assertions
    let sent_packets = helper.collect_received();
    // We skip the packet, because the equipment is empty
    sent_packets.assert_count::<EntityEquipmentUpdateS2c>(0);
}

#[test]
fn test_ensure_living_entities_only() {
    let ScenarioSingleClient {
        mut app,
        mut helper,
        layer,
        ..
    } = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    let zombie_bundle = ZombieEntityBundle {
        layer: EntityLayerId(layer),
        ..Default::default()
    };

    let armor_stand_bundle = ArmorStandEntityBundle {
        layer: EntityLayerId(layer),
        ..Default::default()
    };

    let item_bundle = ItemEntityBundle {
        layer: EntityLayerId(layer),
        ..Default::default()
    };

    let zombie = app
        .world_mut()
        .spawn(zombie_bundle)
        .insert(Equipment::default())
        .id();
    let armor_stand = app
        .world_mut()
        .spawn(armor_stand_bundle)
        .insert(Equipment::default())
        .id();
    let item = app
        .world_mut()
        .spawn(item_bundle)
        .insert(Equipment::default())
        .id();

    app.update();

    let zombie_equipment = app.world_mut().get_mut::<Equipment>(zombie);
    assert!(zombie_equipment.is_some());

    let armor_stand_equipment = app.world_mut().get_mut::<Equipment>(armor_stand);
    assert!(armor_stand_equipment.is_some());

    let item_equipment = app.world_mut().get_mut::<Equipment>(item);
    assert!(item_equipment.is_none());
}
