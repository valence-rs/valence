use bevy_ecs::prelude::{Entity, Event, EventReader};
use bevy_ecs::schedule::IntoSystemConfigs;
use valence_protocol::packets::play::click_slot_c2s::{ClickMode, SlotChange};
use valence_protocol::packets::play::ClickSlotC2s;
use valence_protocol::ItemStack;
use valence_server::event_loop::PacketEvent;

use self::click::{handle_inventory_click, InventoryClickEvent};
use super::*;
use crate::state_event::EventWithStateWriter;

pub mod click;

pub struct InventoryEventPlugin;

impl Plugin for InventoryEventPlugin {
    fn build(&self, app: &mut App) {
        app.add_event_with_state::<ClickSlotEvent>()
            .add_event_with_state::<InventoryClickEvent>()
            .add_systems(
                EventLoopPreUpdate,
                handle_click_slot.in_set(EventDispacherSets::MainEvents),
            )
        .add_systems(
            EventLoopPreUpdate,
            handle_inventory_click.in_set(EventDispacherSets::UserEvents),
        );
    }
}

#[derive(Clone, Debug)]
struct Canceled(bool);

impl State for Canceled {
    fn get(&self) -> bool {
        self.0
    }

    fn set(&mut self, value: bool) {
        self.0 = value;
    }
}

#[derive(Clone, Debug, Event)]
pub struct ClickSlotEvent {
    pub client: Entity,
    pub window_id: u8,
    pub state_id: i32,
    pub slot_idx: i16,
    pub button: i8,
    pub mode: ClickMode,
    pub slot_changes: Vec<SlotChange>,
    pub carried_item: Option<ItemStack>,
}

fn handle_click_slot(
    mut packet_events: EventReader<PacketEvent>,
    mut click_slot_events: EventWithStateWriter<ClickSlotEvent>,
) {
    for packet in packet_events.iter() {
        let Some(pkt) = packet.decode::<ClickSlotC2s>() else {
        continue;
    };

        click_slot_events.send(
            ClickSlotEvent {
                client: packet.client,
                window_id: pkt.window_id,
                state_id: pkt.state_id.0,
                slot_idx: pkt.slot_idx,
                button: pkt.button,
                mode: pkt.mode,
                slot_changes: pkt.slot_changes,
                carried_item: pkt.carried_item,
            },
            Canceled(false),
        );
    }
}
