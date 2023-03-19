use valence_protocol::item::ItemStack;
use valence_protocol::packet::c2s::play::click_slot::ClickMode;
use valence_protocol::packet::c2s::play::ClickSlotC2s;

use super::Inventory;
use crate::prelude::CursorItem;

/// Represents the inventory window that the player is currently viewing.
/// Handles dispatching reads to the correct inventory.
///
/// This is a read-only version of [`InventoryWindowMut`].
struct InventoryWindow<'a> {
    player_inventory: &'a Inventory,
    open_inventory: Option<&'a Inventory>,
}

impl InventoryWindow<'_> {
    #[track_caller]
    pub fn slot(&self, idx: u16) -> Option<&ItemStack> {
        if let Some(open_inv) = self.open_inventory.as_ref() {
            if idx < open_inv.slot_count() {
                return open_inv.slot(idx);
            } else {
                return self
                    .player_inventory
                    .slot(super::convert_to_player_slot_id(open_inv.kind(), idx));
            }
        } else {
            return self.player_inventory.slot(idx);
        }
    }

    #[track_caller]
    pub fn slot_count(&self) -> u16 {
        match self.open_inventory.as_ref() {
            Some(inv) => inv.slot_count() + 36,
            None => self.player_inventory.slot_count(),
        }
    }
}

/// Represents the inventory window that the player is currently viewing.
/// Handles dispatching reads/writes to the correct inventory.
///
/// This is a writable version of [`InventoryWindow`].
struct InventoryWindowMut<'a> {
    player_inventory: &'a mut Inventory,
    open_inventory: Option<&'a mut Inventory>,
}

impl InventoryWindowMut<'_> {
    #[track_caller]
    pub fn slot(&self, idx: u16) -> Option<&ItemStack> {
        if let Some(open_inv) = self.open_inventory.as_ref() {
            if idx < open_inv.slot_count() {
                return open_inv.slot(idx);
            } else {
                return self
                    .player_inventory
                    .slot(super::convert_to_player_slot_id(open_inv.kind(), idx));
            }
        } else {
            return self.player_inventory.slot(idx);
        }
    }

    #[track_caller]
    #[must_use]
    pub fn replace_slot(
        &mut self,
        idx: u16,
        item: impl Into<Option<ItemStack>>,
    ) -> Option<ItemStack> {
        assert!(idx < self.slot_count(), "slot index of {idx} out of bounds");

        if let Some(open_inv) = self.open_inventory.as_mut() {
            if idx < open_inv.slot_count() {
                return open_inv.replace_slot(idx, item);
            } else {
                return self
                    .player_inventory
                    .replace_slot(super::convert_to_player_slot_id(open_inv.kind(), idx), item);
            }
        } else {
            return self.player_inventory.replace_slot(idx, item);
        }
    }

    #[track_caller]
    #[inline]
    pub fn set_slot(&mut self, idx: u16, item: impl Into<Option<ItemStack>>) {
        let _ = self.replace_slot(idx, item);
    }

    pub fn slot_count(&self) -> u16 {
        match self.open_inventory.as_ref() {
            Some(inv) => inv.slot_count() + 36,
            None => self.player_inventory.slot_count(),
        }
    }
}

/// Validates a click slot packet enforcing that all fields are valid.
pub(crate) fn validate_click_slot_impossible(
    packet: &ClickSlotC2s,
    player_inventory: &Inventory,
    open_inventory: Option<&Inventory>,
) -> bool {
    if (packet.window_id == 0) != open_inventory.is_none() {
        return false;
    }

    let max_slot = match open_inventory {
        Some(inv) => inv.slot_count() + 36,
        None => player_inventory.slot_count(),
    };

    if !packet
        .slots
        .iter()
        .all(|s| (0..=max_slot).contains(&(s.idx as u16)))
    {
        return false;
    }

    match packet.mode {
        ClickMode::Click => {
            if !(0..=1).contains(&packet.button) {
                return false;
            }

            if !(0..=max_slot).contains(&(packet.slot_idx as u16)) && packet.slot_idx != -999 {
                return false;
            }
        }
        ClickMode::ShiftClick => {
            if !(0..=1).contains(&packet.button) {
                return false;
            }

            if packet.carried_item.is_some() {
                return false;
            }

            if !(0..=max_slot).contains(&(packet.slot_idx as u16)) {
                return false;
            }
        }
        ClickMode::Hotbar => return matches!(packet.button, 0..=8 | 40),
        ClickMode::CreativeMiddleClick => {
            return packet.button == 2 && (0..=max_slot).contains(&(packet.slot_idx as u16))
        }
        ClickMode::DropKey => {
            return (0..=1).contains(&packet.button)
                && packet.carried_item.is_none()
                && (0..=max_slot).contains(&(packet.slot_idx as u16))
        }
        ClickMode::Drag => {
            return matches!(packet.button, 0..=2 | 4..=6 | 8..=10)
                && ((0..=max_slot).contains(&(packet.slot_idx as u16)) || packet.slot_idx == -999)
        }
        ClickMode::DoubleClick => return packet.button == 0,
    }

    true
}

/// Validates a click slot packet, enforcing that items can't be duplicated, eg.
/// conservation of mass.
pub(crate) fn validate_click_slot_item_duplication(
    packet: &ClickSlotC2s,
    player_inventory: &Inventory,
    open_inventory: Option<&Inventory>,
    cursor_item: &CursorItem,
) -> bool {
    let window = InventoryWindow {
        player_inventory,
        open_inventory,
    };

    match packet.mode {
        ClickMode::Click => {
            if packet.slot_idx == -999 {
                // Clicked outside the window, so the client is dropping an item
                if cursor_item.0.is_none() {
                    // Nothing to drop
                    return false;
                }

                if !packet.slots.is_empty() {
                    return false;
                }

                // Clicked outside the window
                match packet.button {
                    0 => {
                        // drop entire stack
                        if packet.carried_item.is_none() {
                            // Dropping an item
                            return true;
                        }
                    }
                    1 => {
                        // drop single item from stack
                        return match (&cursor_item.0, &packet.carried_item) {
                            (Some(server_item), Some(client_item)) => {
                                server_item.count() - 1 == client_item.count()
                            }
                            (Some(server_item), None) => server_item.count() == 1,
                            (None, _) => {
                                // can't possibly drop an item
                                false
                            }
                        };
                    }
                    _ => {
                        // Invalid button
                        return false;
                    }
                }
                true
            } else {
                if packet.slots.len() != 1 {
                    return false;
                }

                let count_deltas = calculate_net_item_delta(packet, &window, cursor_item);

                return count_deltas == 0;
            }
        }
        ClickMode::ShiftClick => todo!(),
        ClickMode::Hotbar => todo!(),
        ClickMode::CreativeMiddleClick => todo!(),
        ClickMode::DropKey => {
            let count_deltas = calculate_net_item_delta(packet, &window, cursor_item);

            return match packet.button {
                0 => count_deltas == -1,
                1 => {
                    count_deltas
                        == -window
                            .slot(packet.slot_idx as u16)
                            .map(|s| s.count() as i32)
                            .unwrap_or(0)
                }
                _ => unreachable!(),
            };
        }
        ClickMode::Drag => todo!(),
        ClickMode::DoubleClick => todo!(),
    }
}

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

        assert!(validate_click_slot_impossible(
            &packet,
            &player_inventory,
            Some(&inventory)
        ));
        assert!(validate_click_slot_item_duplication(
            &packet,
            &player_inventory,
            Some(&inventory),
            &cursor_item
        ));
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

        assert!(validate_click_slot_impossible(
            &packet1,
            &player_inventory,
            Some(&inventory1),
        ));
        assert!(validate_click_slot_item_duplication(
            &packet1,
            &player_inventory,
            Some(&inventory1),
            &cursor_item
        ));

        assert!(validate_click_slot_impossible(
            &packet2,
            &player_inventory,
            Some(&inventory2)
        ));
        assert!(validate_click_slot_item_duplication(
            &packet2,
            &player_inventory,
            Some(&inventory2),
            &cursor_item
        ));
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

        assert!(validate_click_slot_impossible(
            &packet1,
            &player_inventory,
            Some(&inventory1),
        ));
        assert!(!validate_click_slot_item_duplication(
            &packet1,
            &player_inventory,
            Some(&inventory1),
            &cursor_item
        ));

        assert!(validate_click_slot_impossible(
            &packet2,
            &player_inventory,
            Some(&inventory2)
        ));
        assert!(!validate_click_slot_item_duplication(
            &packet2,
            &player_inventory,
            Some(&inventory2),
            &cursor_item
        ));

        assert!(validate_click_slot_impossible(
            &packet3,
            &player_inventory,
            Some(&inventory1)
        ));
        assert!(!validate_click_slot_item_duplication(
            &packet3,
            &player_inventory,
            Some(&inventory1),
            &cursor_item
        ));
    }
}
