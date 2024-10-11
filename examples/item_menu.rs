#![allow(clippy::type_complexity)]

const SPAWN_Y: i32 = 64;

use item_menu::{ItemMenu, ItemMenuPlugin, MenuItemSelectEvent};
use valence::interact_item::InteractItemEvent;
use valence::prelude::*;
use valence::protocol::sound::SoundCategory;
use valence::protocol::Sound;
use valence_inventory::HeldItem;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ItemMenuPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                init_clients,
                despawn_disconnected_clients,
                on_item_interact,
                on_make_selection,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            layer
                .chunk
                .set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(layer);
}

fn init_clients(
    mut clients: Query<
        (
            &mut Position,
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut GameMode,
            &mut Inventory,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for (
        mut pos,
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut game_mode,
        mut inventory,
    ) in &mut clients
    {
        let layer = layers.single();

        pos.0 = [0.0, f64::from(SPAWN_Y) + 1.0, 0.0].into();
        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        *game_mode = GameMode::Survival;

        // 40 is the fifth hotbar slot
        inventory.set_slot(40, ItemStack::new(ItemKind::Compass, 1, None));
    }
}

fn on_item_interact(
    mut commands: Commands,
    clients: Query<(Entity, &HeldItem, &Inventory)>,
    mut events: EventReader<InteractItemEvent>,
) {
    for event in events.read() {
        let Ok((player_ent, held_item, inventory)) = clients.get(event.client) else {
            continue;
        };
        if *inventory.slot(held_item.slot()) == ItemStack::new(ItemKind::Compass, 1, None) {
            open_menu(&mut commands, player_ent);
        }
    }
}

fn open_menu(commands: &mut Commands, player: Entity) {
    let mut menu_inv = Inventory::new(InventoryKind::Generic3x3);

    menu_inv.set_slot(3, ItemStack::new(ItemKind::RedWool, 1, None));
    menu_inv.set_slot(5, ItemStack::new(ItemKind::GreenWool, 1, None));

    let menu = ItemMenu::new(menu_inv);
    commands.entity(player).insert(menu);
}

fn on_make_selection(
    mut clients: Query<(&mut Client, &Position)>,
    mut events: EventReader<MenuItemSelectEvent>,
) {
    for event in events.read() {
        let Ok((mut client, pos)) = clients.get_mut(event.client) else {
            continue;
        };

        let selected_color = match event.idx {
            3 => "§cRED",
            5 => "§aGREEN",
            _ => continue,
        };

        client.play_sound(
            Sound::BlockNoteBlockBit,
            SoundCategory::Block,
            pos.0,
            1.0,
            1.0,
        );
        client.send_chat_message(format!("you clicked: {}", selected_color));
    }
}

mod item_menu {
    use valence::prelude::*;
    use valence_inventory::ClickSlotEvent;

    pub(crate) struct ItemMenuPlugin;

    impl Plugin for ItemMenuPlugin {
        fn build(&self, app: &mut App) {
            app.add_systems(Update, (open_menu, select_menu_item))
                .add_event::<MenuItemSelectEvent>()
                .observe(close_menu);
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Event)]
    pub(crate) struct MenuItemSelectEvent {
        /// Player entity
        pub client: Entity,
        /// Index of the item in the menu
        pub idx: u16,
    }

    #[derive(Debug, Clone, Component)]
    pub(crate) struct ItemMenu {
        /// Item menu
        pub menu: Inventory,
    }

    impl ItemMenu {
        pub(crate) fn new(mut menu: Inventory) -> Self {
            menu.readonly = true;
            Self { menu }
        }
    }

    fn open_menu(
        mut commands: Commands,
        mut clients: Query<(Entity, &mut ItemMenu), Added<ItemMenu>>,
    ) {
        for (player, item_menu) in clients.iter_mut() {
            let inventory = commands.spawn(item_menu.menu.clone()).id();

            commands
                .entity(player)
                .insert(OpenInventory::new(inventory));
        }
    }

    fn close_menu(
        _trigger: Trigger<OnRemove, OpenInventory>,
        mut commands: Commands,
        clients: Query<Entity, With<ItemMenu>>,
    ) {
        for player in clients.iter() {
            commands.entity(player).remove::<ItemMenu>();
        }
    }

    fn select_menu_item(
        mut clients: Query<(Entity, &ItemMenu)>,
        mut events: EventReader<ClickSlotEvent>,
        mut event_writer: EventWriter<MenuItemSelectEvent>,
    ) {
        for event in events.read() {
            let selected_slot = event.slot_id;
            let Ok((player, item_menu)) = clients.get_mut(event.client) else {
                continue;
            };
            // check that the selected item is not in the player's own inventory
            if selected_slot as u16 >= item_menu.menu.slot_count() {
                continue;
            }

            event_writer.send(MenuItemSelectEvent {
                client: player,
                idx: selected_slot as u16,
            });
        }
    }
}
