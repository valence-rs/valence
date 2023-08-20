use bevy_ecs::prelude::EventWriter;

use super::*;

#[derive(Debug, Event, Clone)]
pub struct InventoryClickEvent {
    pub client: Entity,
    pub window_id: u8,
    pub state_id: i32,
    pub slot_idx: i16,
    pub click_type: ClickType,
    pub clicked_item: Option<SlotChange>,
    pub cursor_item: Option<ItemStack>,
}

#[derive(Debug, Clone)]
pub enum ClickType {
    Left,
    Right,
    ShiftLeft,
    ShiftRight,
    DoubleLeft,
    DoubleRight,
}

pub(super) fn handle_inventory_click(
    mut click_slot_events: EventWithStateReader<ClickSlotEvent>,
    mut inventory_click_events: EventWriter<InventoryClickEvent>,
) {
    for (event, _) in click_slot_events.iter_some(EventState::new().with_canceled(false)) {
        let click_type = match event.mode {
            ClickMode::Click => match event.button {
                0 => ClickType::Left,
                1 => ClickType::Right,
                _ => panic!("Invalid button for click mode"),
            },
            ClickMode::ShiftClick => match event.button {
                0 => ClickType::ShiftLeft,
                1 => ClickType::ShiftRight,
                _ => panic!("Invalid button for shift click mode"),
            },
            ClickMode::DoubleClick => match event.button {
                0 => ClickType::DoubleLeft,
                1 => ClickType::DoubleRight,
                _ => panic!("Invalid button for double click mode"),
            },
            _ => {
                continue;
            }
        };

        let inventory_click_event = InventoryClickEvent {
            client: event.client,
            window_id: event.window_id,
            state_id: event.state_id,
            slot_idx: event.slot_idx,
            click_type,
            clicked_item: event.slot_changes.get(0).cloned(),
            cursor_item: event.carried_item.clone(),
        };
        
        inventory_click_events.send(inventory_click_event);
    }
}
