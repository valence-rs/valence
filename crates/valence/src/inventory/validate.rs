use anyhow::ensure;
use valence_protocol::packet::c2s::play::click_slot::ClickMode;
use valence_protocol::packet::c2s::play::ClickSlotC2s;

use super::{Inventory, InventoryWindow, PLAYER_INVENTORY_MAIN_SLOTS_COUNT};
use crate::prelude::CursorItem;

/// Validates a click slot packet enforcing that all fields are valid.
pub(crate) fn validate_click_slot_impossible(
    packet: &ClickSlotC2s,
    player_inventory: &Inventory,
    open_inventory: Option<&Inventory>,
    cursor_item: &CursorItem,
) -> anyhow::Result<()> {
    ensure!(
        (packet.window_id == 0) == open_inventory.is_none(),
        "window id and open inventory mismatch: window_id: {} open_inventory: {}",
        packet.window_id,
        open_inventory.is_some()
    );

    let max_slot = match open_inventory {
        Some(inv) => inv.slot_count() + PLAYER_INVENTORY_MAIN_SLOTS_COUNT,
        None => player_inventory.slot_count(),
    };

    let window = InventoryWindow::new(player_inventory, open_inventory);

    // check all slot ids and item counts are valid
    ensure!(
        packet.slots.iter().all(|s| {
            if !(0..=max_slot).contains(&(s.idx as u16)) {
                return false;
            }
            if let Some(slot) = s.item.as_ref() {
                let max_stack_size = slot.item.max_stack().max(slot.count());
                if !(1..=max_stack_size).contains(&slot.count()) {
                    return false;
                }
            }

            true
        }),
        "invalid slot ids or item counts"
    );

    // check carried item count is valid
    if let Some(carried_item) = &packet.carried_item {
        let max_stack_size = carried_item.item.max_stack().max(carried_item.count());
        ensure!(
            (1..=max_stack_size).contains(&carried_item.count()),
            "invalid carried item count"
        );
    }

    match packet.mode {
        ClickMode::Click => {
            ensure!((0..=1).contains(&packet.button), "invalid button");
            ensure!(
                (0..=max_slot).contains(&(packet.slot_idx as u16)) || packet.slot_idx == -999,
                "invalid slot index"
            )
        }
        ClickMode::ShiftClick => {
            ensure!((0..=1).contains(&packet.button), "invalid button");
            ensure!(
                packet.carried_item.is_none(),
                "carried item must be empty for a hotbar swap"
            );
            ensure!(
                (0..=max_slot).contains(&(packet.slot_idx as u16)),
                "invalid slot index"
            )
        }
        ClickMode::Hotbar => {
            ensure!(matches!(packet.button, 0..=8 | 40), "invalid button");
            ensure!(
                packet.carried_item.is_none(),
                "carried item must be empty for a hotbar swap"
            );
        }
        ClickMode::CreativeMiddleClick => {
            ensure!(packet.button == 2, "invalid button");
            ensure!(
                (0..=max_slot).contains(&(packet.slot_idx as u16)),
                "invalid slot index"
            )
        }
        ClickMode::DropKey => {
            ensure!((0..=1).contains(&packet.button), "invalid button");
            ensure!(
                packet.carried_item.is_none(),
                "carried item must be empty for an item drop"
            );
            ensure!(
                (0..=max_slot).contains(&(packet.slot_idx as u16)),
                "invalid slot index"
            )
        }
        ClickMode::Drag => {
            ensure!(
                matches!(packet.button, 0..=2 | 4..=6 | 8..=10),
                "invalid button"
            );
            ensure!(
                (0..=max_slot).contains(&(packet.slot_idx as u16)) || packet.slot_idx == -999,
                "invalid slot index"
            )
        }
        ClickMode::DoubleClick => ensure!(packet.button == 0, "invalid button"),
    }

    Ok(())
}

/// Validates a click slot packet, enforcing that items can't be duplicated, eg.
/// conservation of mass.
///
/// Relies on assertions made by [`validate_click_slot_impossible`].
pub(crate) fn validate_click_slot_item_duplication(
    packet: &ClickSlotC2s,
    player_inventory: &Inventory,
    open_inventory: Option<&Inventory>,
    cursor_item: &CursorItem,
) -> anyhow::Result<()> {
    let window = InventoryWindow {
        player_inventory,
        open_inventory,
    };

    match packet.mode {
        ClickMode::Click => {
            if packet.slot_idx == -999 {
                // Clicked outside the window, so the client is dropping an item
                ensure!(packet.slots.is_empty(), "slot modifications must be empty");

                // Clicked outside the window
                let count_deltas = calculate_net_item_delta(packet, &window, cursor_item);
                let expected_delta = match packet.button {
                    1 => -1,
                    0 => -cursor_item
                        .0
                        .as_ref()
                        .map(|s| s.count() as i32)
                        .unwrap_or(0),
                    _ => unreachable!(),
                };
                ensure!(
                    count_deltas == expected_delta,
                    "invalid item delta: expected {}, got {}",
                    expected_delta,
                    count_deltas
                );
            } else {
                ensure!(
                    packet.slots.len() == 1,
                    "click must modify one slot, got {}",
                    packet.slots.len()
                );

                let old_slot = window.slot(packet.slots[0].idx as u16);
                // TODO: make sure NBT is the same
                // Sometimes, the client will add nbt data to an item if it's missing, like
                // "Damage" to a sword
                if packet.button == 0
                    && (old_slot.is_none()
                        || cursor_item.0.is_none()
                        || old_slot.unwrap().item != cursor_item.0.as_ref().unwrap().item)
                {
                    // assert that a swap occurs
                    ensure!(
                        old_slot == packet.carried_item.as_ref()
                            && cursor_item.0 == packet.slots[0].item,
                        "swapped items must match"
                    );
                } else {
                    // assert that a merge occurs
                    let count_deltas = calculate_net_item_delta(packet, &window, cursor_item);
                    ensure!(
                        count_deltas == 0,
                        "invalid item delta for stack merge: {}",
                        count_deltas
                    );
                }
            }
        }
        ClickMode::ShiftClick | ClickMode::Hotbar => {
            ensure!(
                packet.slots.len() == 2,
                "shift click and hotbar swaps must modify exactly two slots"
            );

            let count_deltas = calculate_net_item_delta(packet, &window, cursor_item);
            ensure!(
                count_deltas == 0,
                "invalid item delta: expected 0, got {}",
                count_deltas
            );

            // assert that a swap occurs
            let old_slots = [
                window.slot(packet.slots[0].idx as u16),
                window.slot(packet.slots[1].idx as u16),
            ];
            ensure!(
                old_slots
                    .iter()
                    .any(|s| s == &packet.slots[0].item.as_ref())
                    && old_slots
                        .iter()
                        .any(|s| s == &packet.slots[1].item.as_ref()),
                "swapped items must match"
            );
        }
        ClickMode::CreativeMiddleClick => {}
        ClickMode::DropKey => {
            ensure!(
                packet.slots.len() == 1,
                "drop key must modify exactly one slot"
            );
            ensure!(
                packet.slot_idx == packet.slots.first().map(|s| s.idx).unwrap_or(-2),
                "slot index does not match modified slot"
            );

            let old_slot = window.slot(packet.slot_idx as u16);
            let new_slot = packet.slots[0].item.as_ref();
            let is_transmuting = match (old_slot, new_slot) {
                // TODO: make sure NBT is the same
                // Sometimes, the client will add nbt data to an item if it's missing, like "Damage"
                // to a sword
                (Some(old_slot), Some(new_slot)) => old_slot.item != new_slot.item,
                (_, None) => false,
                (None, Some(_)) => true,
            };
            ensure!(!is_transmuting, "transmuting items is not allowed");

            let count_deltas = calculate_net_item_delta(packet, &window, cursor_item);

            let expected_delta = match packet.button {
                0 => -1,
                1 => -old_slot.map(|s| s.count() as i32).unwrap_or(0),
                _ => unreachable!(),
            };
            ensure!(
                count_deltas == expected_delta,
                "invalid item delta: expected {}, got {}",
                expected_delta,
                count_deltas
            );
        }
        ClickMode::Drag => {
            if matches!(packet.button, 2 | 6 | 10) {
                let count_deltas = calculate_net_item_delta(packet, &window, cursor_item);
                ensure!(
                    count_deltas == 0,
                    "invalid item delta: expected 0, got {}",
                    count_deltas
                );
            } else {
                ensure!(packet.slots.is_empty() && packet.carried_item == cursor_item.0);
            }
        }
        ClickMode::DoubleClick => {
            let count_deltas = calculate_net_item_delta(packet, &window, cursor_item);
            ensure!(
                count_deltas == 0,
                "invalid item delta: expected 0, got {}",
                count_deltas
            );
        }
    }

    Ok(())
}

/// Calculate the total difference in item counts if the changes in this packet
/// were to be applied.
///
/// Returns a positive number if items were added to the window, and a negative
/// number if items were removed from the window.
fn calculate_net_item_delta(
    packet: &ClickSlotC2s,
    window: &InventoryWindow,
    cursor_item: &CursorItem,
) -> i32 {
    let mut net_item_delta: i32 = 0;

    for slot in &packet.slots {
        let old_slot = window.slot(slot.idx as u16);
        let new_slot = slot.item.as_ref();

        net_item_delta += match (old_slot, new_slot) {
            (Some(old), Some(new)) => new.count() as i32 - old.count() as i32,
            (Some(old), None) => -(old.count() as i32),
            (None, Some(new)) => new.count() as i32,
            (None, None) => 0,
        };
    }

    net_item_delta += match (cursor_item.0.as_ref(), packet.carried_item.as_ref()) {
        (Some(old), Some(new)) => new.count() as i32 - old.count() as i32,
        (Some(old), None) => -(old.count() as i32),
        (None, Some(new)) => new.count() as i32,
        (None, None) => 0,
    };

    net_item_delta
}

#[cfg(test)]
mod test {
    use valence_protocol::item::{ItemKind, ItemStack};
    use valence_protocol::packet::c2s::play::click_slot::Slot;
    use valence_protocol::var_int::VarInt;

    use super::*;
    use crate::prelude::InventoryKind;

    #[test]
    fn net_item_delta_1() {
        let drag_packet = ClickSlotC2s {
            window_id: 2,
            state_id: VarInt(14),
            slot_idx: -999,
            button: 2,
            mode: ClickMode::Drag,
            slots: vec![
                Slot {
                    idx: 4,
                    item: Some(ItemStack::new(ItemKind::Diamond, 21, None)),
                },
                Slot {
                    idx: 3,
                    item: Some(ItemStack::new(ItemKind::Diamond, 21, None)),
                },
                Slot {
                    idx: 5,
                    item: Some(ItemStack::new(ItemKind::Diamond, 21, None)),
                },
            ],
            carried_item: Some(ItemStack::new(ItemKind::Diamond, 1, None)),
        };

        let player_inventory = Inventory::new(InventoryKind::Player);
        let inventory = Inventory::new(InventoryKind::Generic9x1);
        let window = InventoryWindow::new(&player_inventory, Some(&inventory));
        let cursor_item = CursorItem(Some(ItemStack::new(ItemKind::Diamond, 64, None)));

        assert_eq!(
            calculate_net_item_delta(&drag_packet, &window, &cursor_item),
            0
        );
    }

    #[test]
    fn net_item_delta_2() {
        let drag_packet = ClickSlotC2s {
            window_id: 2,
            state_id: VarInt(14),
            slot_idx: -999,
            button: 2,
            mode: ClickMode::Click,
            slots: vec![
                Slot {
                    idx: 2,
                    item: Some(ItemStack::new(ItemKind::Diamond, 2, None)),
                },
                Slot {
                    idx: 3,
                    item: Some(ItemStack::new(ItemKind::IronIngot, 2, None)),
                },
                Slot {
                    idx: 4,
                    item: Some(ItemStack::new(ItemKind::GoldIngot, 2, None)),
                },
                Slot {
                    idx: 5,
                    item: Some(ItemStack::new(ItemKind::Emerald, 2, None)),
                },
            ],
            carried_item: Some(ItemStack::new(ItemKind::OakWood, 2, None)),
        };

        let player_inventory = Inventory::new(InventoryKind::Player);
        let inventory = Inventory::new(InventoryKind::Generic9x1);
        let window = InventoryWindow::new(&player_inventory, Some(&inventory));
        let cursor_item = CursorItem::default();

        assert_eq!(
            calculate_net_item_delta(&drag_packet, &window, &cursor_item),
            10
        );
    }

    #[test]
    fn click_filled_slot_with_empty_cursor_success() {
        let player_inventory = Inventory::new(InventoryKind::Player);
        let mut inventory = Inventory::new(InventoryKind::Generic9x1);
        inventory.set_slot(0, ItemStack::new(ItemKind::Diamond, 20, None));
        let cursor_item = CursorItem::default();
        let packet = ClickSlotC2s {
            window_id: 1,
            button: 0,
            mode: ClickMode::Click,
            state_id: VarInt(0),
            slot_idx: 0,
            slots: vec![Slot { idx: 0, item: None }],
            carried_item: inventory.slot(0).cloned(),
        };

        validate_click_slot_impossible(&packet, &player_inventory, Some(&inventory), &cursor_item)
            .expect("packet should be valid");
        validate_click_slot_item_duplication(
            &packet,
            &player_inventory,
            Some(&inventory),
            &cursor_item,
        )
        .expect("packet should not fail item duplication check");
    }

    #[test]
    fn click_slot_with_filled_cursor_success() {
        let player_inventory = Inventory::new(InventoryKind::Player);
        let inventory1 = Inventory::new(InventoryKind::Generic9x1);
        let mut inventory2 = Inventory::new(InventoryKind::Generic9x1);
        inventory2.set_slot(0, ItemStack::new(ItemKind::Diamond, 10, None));
        let cursor_item = CursorItem(Some(ItemStack::new(ItemKind::Diamond, 20, None)));
        let packet1 = ClickSlotC2s {
            window_id: 1,
            button: 0,
            mode: ClickMode::Click,
            state_id: VarInt(0),
            slot_idx: 0,
            slots: vec![Slot {
                idx: 0,
                item: Some(ItemStack::new(ItemKind::Diamond, 20, None)),
            }],
            carried_item: None,
        };
        let packet2 = ClickSlotC2s {
            window_id: 1,
            button: 0,
            mode: ClickMode::Click,
            state_id: VarInt(0),
            slot_idx: 0,
            slots: vec![Slot {
                idx: 0,
                item: Some(ItemStack::new(ItemKind::Diamond, 30, None)),
            }],
            carried_item: None,
        };

        validate_click_slot_impossible(
            &packet1,
            &player_inventory,
            Some(&inventory1),
            &cursor_item,
        )
        .expect("packet should be valid");
        validate_click_slot_item_duplication(
            &packet1,
            &player_inventory,
            Some(&inventory1),
            &cursor_item,
        )
        .expect("packet should not fail item duplication check");

        validate_click_slot_impossible(
            &packet2,
            &player_inventory,
            Some(&inventory2),
            &cursor_item,
        )
        .expect("packet should be valid");
        validate_click_slot_item_duplication(
            &packet2,
            &player_inventory,
            Some(&inventory2),
            &cursor_item,
        )
        .expect("packet should not fail item duplication check");
    }

    #[test]
    fn click_filled_slot_with_filled_cursor_stack_overflow_success() {
        let player_inventory = Inventory::new(InventoryKind::Player);
        let mut inventory = Inventory::new(InventoryKind::Generic9x1);
        inventory.set_slot(0, ItemStack::new(ItemKind::Diamond, 20, None));
        let cursor_item = CursorItem(Some(ItemStack::new(ItemKind::Diamond, 64, None)));
        let packet = ClickSlotC2s {
            window_id: 1,
            button: 0,
            mode: ClickMode::Click,
            state_id: VarInt(0),
            slot_idx: 0,
            slots: vec![Slot {
                idx: 0,
                item: Some(ItemStack::new(ItemKind::Diamond, 64, None)),
            }],
            carried_item: Some(ItemStack::new(ItemKind::Diamond, 20, None)),
        };

        validate_click_slot_impossible(&packet, &player_inventory, Some(&inventory), &cursor_item)
            .expect("packet should be valid");
        validate_click_slot_item_duplication(
            &packet,
            &player_inventory,
            Some(&inventory),
            &cursor_item,
        )
        .expect("packet should not fail item duplication check");
    }

    #[test]
    fn click_filled_slot_with_filled_cursor_different_item_success() {
        let player_inventory = Inventory::new(InventoryKind::Player);
        let mut inventory = Inventory::new(InventoryKind::Generic9x1);
        inventory.set_slot(0, ItemStack::new(ItemKind::IronIngot, 2, None));
        let cursor_item = CursorItem(Some(ItemStack::new(ItemKind::Diamond, 2, None)));
        let packet = ClickSlotC2s {
            window_id: 1,
            button: 0,
            mode: ClickMode::Click,
            state_id: VarInt(0),
            slot_idx: 0,
            slots: vec![Slot {
                idx: 0,
                item: Some(ItemStack::new(ItemKind::Diamond, 2, None)),
            }],
            carried_item: Some(ItemStack::new(ItemKind::IronIngot, 2, None)),
        };

        validate_click_slot_impossible(&packet, &player_inventory, Some(&inventory), &cursor_item)
            .expect("packet should be valid");
        validate_click_slot_item_duplication(
            &packet,
            &player_inventory,
            Some(&inventory),
            &cursor_item,
        )
        .expect("packet should not fail item duplication check");
    }

    #[test]
    fn click_slot_with_filled_cursor_failure() {
        let player_inventory = Inventory::new(InventoryKind::Player);
        let inventory1 = Inventory::new(InventoryKind::Generic9x1);
        let mut inventory2 = Inventory::new(InventoryKind::Generic9x1);
        inventory2.set_slot(0, ItemStack::new(ItemKind::Diamond, 10, None));
        let cursor_item = CursorItem(Some(ItemStack::new(ItemKind::Diamond, 20, None)));
        let packet1 = ClickSlotC2s {
            window_id: 1,
            button: 0,
            mode: ClickMode::Click,
            state_id: VarInt(0),
            slot_idx: 0,
            slots: vec![Slot {
                idx: 0,
                item: Some(ItemStack::new(ItemKind::Diamond, 22, None)),
            }],
            carried_item: None,
        };
        let packet2 = ClickSlotC2s {
            window_id: 1,
            button: 0,
            mode: ClickMode::Click,
            state_id: VarInt(0),
            slot_idx: 0,
            slots: vec![Slot {
                idx: 0,
                item: Some(ItemStack::new(ItemKind::Diamond, 32, None)),
            }],
            carried_item: None,
        };
        let packet3 = ClickSlotC2s {
            window_id: 1,
            button: 0,
            mode: ClickMode::Click,
            state_id: VarInt(0),
            slot_idx: 0,
            slots: vec![
                Slot {
                    idx: 0,
                    item: Some(ItemStack::new(ItemKind::Diamond, 22, None)),
                },
                Slot {
                    idx: 1,
                    item: Some(ItemStack::new(ItemKind::Diamond, 22, None)),
                },
            ],
            carried_item: None,
        };

        validate_click_slot_impossible(
            &packet1,
            &player_inventory,
            Some(&inventory1),
            &cursor_item,
        )
        .expect("packet 1 should be valid");
        validate_click_slot_item_duplication(
            &packet1,
            &player_inventory,
            Some(&inventory1),
            &cursor_item,
        )
        .expect_err("packet 1 should fail item duplication check");

        validate_click_slot_impossible(
            &packet2,
            &player_inventory,
            Some(&inventory2),
            &cursor_item,
        )
        .expect("packet 2 should be valid");
        validate_click_slot_item_duplication(
            &packet2,
            &player_inventory,
            Some(&inventory2),
            &cursor_item,
        )
        .expect_err("packet 2 should fail item duplication check");

        validate_click_slot_impossible(
            &packet3,
            &player_inventory,
            Some(&inventory1),
            &cursor_item,
        )
        .expect("packet 3 should be valid");
        validate_click_slot_item_duplication(
            &packet3,
            &player_inventory,
            Some(&inventory1),
            &cursor_item,
        )
        .expect_err("packet 3 should fail item duplication check");
    }

    #[test]
    fn disallow_item_transmutation() {
        // no alchemy allowed - make sure that lead can't be turned into gold

        let mut player_inventory = Inventory::new(InventoryKind::Player);
        player_inventory.set_slot(9, ItemStack::new(ItemKind::Lead, 2, None));
        let cursor_item = CursorItem::default();

        let packets = vec![
            ClickSlotC2s {
                window_id: 0,
                button: 0,
                mode: ClickMode::ShiftClick,
                state_id: VarInt(0),
                slot_idx: 9,
                slots: vec![
                    Slot { idx: 9, item: None },
                    Slot {
                        idx: 36,
                        item: Some(ItemStack::new(ItemKind::GoldIngot, 2, None)),
                    },
                ],
                carried_item: None,
            },
            ClickSlotC2s {
                window_id: 0,
                button: 0,
                mode: ClickMode::Hotbar,
                state_id: VarInt(0),
                slot_idx: 9,
                slots: vec![
                    Slot { idx: 9, item: None },
                    Slot {
                        idx: 36,
                        item: Some(ItemStack::new(ItemKind::GoldIngot, 2, None)),
                    },
                ],
                carried_item: None,
            },
            ClickSlotC2s {
                window_id: 0,
                button: 0,
                mode: ClickMode::Click,
                state_id: VarInt(0),
                slot_idx: 9,
                slots: vec![Slot { idx: 9, item: None }],
                carried_item: Some(ItemStack::new(ItemKind::GoldIngot, 2, None)),
            },
            ClickSlotC2s {
                window_id: 0,
                button: 0,
                mode: ClickMode::DropKey,
                state_id: VarInt(0),
                slot_idx: 9,
                slots: vec![Slot {
                    idx: 9,
                    item: Some(ItemStack::new(ItemKind::GoldIngot, 1, None)),
                }],
                carried_item: None,
            },
        ];

        for (i, packet) in packets.iter().enumerate() {
            validate_click_slot_impossible(packet, &player_inventory, None, &cursor_item)
                .unwrap_or_else(|e| panic!("packet {i} should be valid: {e}"));
            validate_click_slot_item_duplication(packet, &player_inventory, None, &cursor_item)
                .expect_err(&format!(
                    "packet {i} passed item duplication check when it should have failed"
                ));
        }
    }
}