use std::borrow::Cow;

use bevy_app::{CoreSet, IntoSystemAppConfig, Plugin};
use bevy_ecs::prelude::*;
use tracing::debug;
use valence_client::event_loop::{EventLoopSchedule, PacketEvent};
use valence_client::Client;
use valence_core::__private::VarInt;
use valence_core::item::ItemStack;
use valence_core::protocol::encode::WritePacket;
use valence_core::text::Text;

use crate::packet::{
    ClickMode, ClickSlotC2s, InventoryS2c, OpenScreenS2c, ScreenHandlerSlotUpdateS2c, WindowType,
};
use crate::{
    validate, ClickSlot, ClientInventoryState, CursorItem, DropItemStack, Inventory, InventoryKind,
    OpenInventory,
};

#[derive(Debug, Default)]
pub struct InventoryMenuPlugin;

impl Plugin for InventoryMenuPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_system(
            handle_click_slot
                .in_schedule(EventLoopSchedule)
                .in_base_set(CoreSet::PreUpdate),
        )
        .add_event::<MenuClickEvent>();
    }
}

#[derive(Debug, Component)]
pub struct OpenMenu;

#[derive(Debug, Component)]
pub struct InventoryMenu;

fn handle_click_slot(
    mut clicks: EventReader<ClickSlot>,
    mut clients: Query<
        (
            &mut Client,
            &mut Inventory,
            &mut ClientInventoryState,
            &mut CursorItem,
            &mut OpenInventory,
        ),
        With<OpenMenu>,
    >,
    mut inventories: Query<&mut Inventory, (Without<Client>, With<InventoryMenu>)>,
    mut menu_clicks: EventWriter<MenuClickEvent>,
) {
    for click in clicks.iter() {
        if click.mode != ClickMode::Click {
            continue;
        }
        println!("menu click: {:?}", click);
        if let Ok((mut client, mut inv, mut inv_state, mut cursor_item, open_inv)) =
            clients.get_mut(click.client)
        {
            let Ok(mut target) = inventories.get_mut(open_inv.entity) else {
                continue;
            };
            if click.slot_id < 0 || click.slot_id >= (target.slot_count() as i16) {
                continue;
            }
            println!("reset clicked slot");
            target.set_slot(click.slot_id as u16, click.carried_item.clone());
            inv_state.slots_changed &= !(1 << click.slot_id);
            client.write_packet(&ScreenHandlerSlotUpdateS2c {
                window_id: inv_state.window_id as i8,
                state_id: VarInt(inv_state.state_id.0),
                slot_idx: click.slot_id,
                slot_data: Cow::Borrowed(&click.carried_item),
            });

            println!("clearing cursor item");
            cursor_item.0 = None;
            inv_state.client_updated_cursor_item = false;

            menu_clicks.send(MenuClickEvent {
                client: click.client,
                slot_id: click.slot_id as u16,
                button: click.button,
            });
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuClickEvent {
    pub client: Entity,
    pub slot_id: u16,
    pub button: i8,
}
