use crate::ItemKind;

#[test]
fn test_serialize_item_kind() {
    let item = ItemKind::WhiteWool;
    let serialized = serde_json::to_string(&item).unwrap();
    assert_eq!(serialized, "\"white_wool\"");

    let item = ItemKind::GoldenSword;
    let serialized = serde_json::to_string(&item).unwrap();
    assert_eq!(serialized, "\"golden_sword\"");

    let item = ItemKind::NetheriteAxe;
    let serialized = serde_json::to_string(&item).unwrap();
    assert_eq!(serialized, "\"netherite_axe\"");

    let item = ItemKind::WaxedWeatheredCutCopperStairs;
    let serialized = serde_json::to_string(&item).unwrap();
    assert_eq!(serialized, "\"waxed_weathered_cut_copper_stairs\"");
}

#[test]
fn test_deserialize_item_kind() {
    let id = "\"diamond_shovel\"";
    let deserialized: ItemKind = serde_json::from_str(id).unwrap();
    assert_eq!(deserialized, ItemKind::DiamondShovel);

    let id = "\"minecart\"";
    let deserialized: ItemKind = serde_json::from_str(id).unwrap();
    assert_eq!(deserialized, ItemKind::Minecart);

    let id = "\"vindicator_spawn_egg\"";
    let deserialized: ItemKind = serde_json::from_str(id).unwrap();
    assert_eq!(deserialized, ItemKind::VindicatorSpawnEgg);
}
