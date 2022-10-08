use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use num::Integer;
use valence::async_trait;
use valence::block::BlockState;
use valence::chunk::{Chunk, UnloadedChunk};
use valence::client::{handle_event_default, ClientEvent, Hand};
use valence::config::{Config, ServerListPing};
use valence::dimension::{Dimension, DimensionId};
use valence::entity::{EntityId, EntityKind};
use valence::inventory::{
    ConfigurableInventory, Inventory, InventoryId, PlayerInventory, WindowInventory,
};
use valence::itemstack::ItemStack;
use valence::player_list::PlayerListId;
use valence::protocol::packets::s2c::play::OpenScreen;
use valence::protocol::{Slot, SlotId, VarInt};
use valence::server::{Server, SharedServer, ShutdownResult};
use valence::text::{Color, TextFormat};

pub fn main() -> ShutdownResult {
    env_logger::Builder::new()
        .filter_module("valence", LevelFilter::Trace)
        .parse_default_env()
        .init();

    valence::start_server(
        Game {
            player_count: AtomicUsize::new(0),
        },
        ServerState {
            player_list: None,
            chest: Default::default(),
            tick: 0,
        },
    )
}

struct Game {
    player_count: AtomicUsize,
}

struct ServerState {
    player_list: Option<PlayerListId>,
    chest: InventoryId,
    tick: u32,
}

#[derive(Default)]
struct ClientState {
    entity_id: EntityId,
    // open_inventory: Option<WindowInventory>,
}

const MAX_PLAYERS: usize = 10;

const SIZE_X: usize = 100;
const SIZE_Z: usize = 100;

#[async_trait]
impl Config for Game {
    type ServerState = ServerState;
    type ClientState = ClientState;
    type EntityState = ();
    type WorldState = ();
    type ChunkState = ();
    type PlayerListState = ();

    fn max_connections(&self) -> usize {
        // We want status pings to be successful even if the server is full.
        MAX_PLAYERS + 64
    }

    fn dimensions(&self) -> Vec<Dimension> {
        vec![Dimension {
            fixed_time: Some(6000),
            ..Dimension::default()
        }]
    }

    async fn server_list_ping(
        &self,
        _server: &SharedServer<Self>,
        _remote_addr: SocketAddr,
        _protocol_version: i32,
    ) -> ServerListPing {
        ServerListPing::Respond {
            online_players: self.player_count.load(Ordering::SeqCst) as i32,
            max_players: MAX_PLAYERS as i32,
            player_sample: Default::default(),
            description: "Hello Valence!".color(Color::AQUA),
            favicon_png: Some(include_bytes!("../assets/logo-64x64.png").as_slice().into()),
        }
    }

    fn init(&self, server: &mut Server<Self>) {
        let world = server.worlds.insert(DimensionId::default(), ()).1;
        server.state.player_list = Some(server.player_lists.insert(()).0);

        // initialize chunks
        for chunk_z in -2..Integer::div_ceil(&(SIZE_Z as i32), &16) + 2 {
            for chunk_x in -2..Integer::div_ceil(&(SIZE_X as i32), &16) + 2 {
                world.chunks.insert(
                    [chunk_x as i32, chunk_z as i32],
                    UnloadedChunk::default(),
                    (),
                );
            }
        }

        // initialize blocks in the chunks
        for chunk_x in 0..Integer::div_ceil(&SIZE_X, &16) {
            for chunk_z in 0..Integer::div_ceil(&SIZE_Z, &16) {
                let chunk = world
                    .chunks
                    .get_mut((chunk_x as i32, chunk_z as i32))
                    .unwrap();
                for x in 0..16 {
                    for z in 0..16 {
                        let cell_x = chunk_x * 16 + x;
                        let cell_z = chunk_z * 16 + z;

                        if cell_x < SIZE_X && cell_z < SIZE_Z {
                            chunk.set_block_state(x, 63, z, BlockState::GRASS_BLOCK);
                        }
                    }
                }
            }
        }

        world
            .chunks
            .set_block_state((50, -1, 54), BlockState::STONE);
        world.chunks.set_block_state((50, 0, 54), BlockState::CHEST);

        // create chest inventory
        let inv = ConfigurableInventory::new(27, VarInt(2), None);
        let (id, _inv) = server.inventories.insert(inv);
        server.state.chest = id;
    }

    fn update(&self, server: &mut Server<Self>) {
        server.state.tick += 1;
        if server.state.tick > 10 {
            server.state.tick = 0;
        }
        let (world_id, world) = server.worlds.iter_mut().next().unwrap();

        let spawn_pos = [SIZE_X as f64 / 2.0, 1.0, SIZE_Z as f64 / 2.0];

        if let Some(inv) = server.inventories.get_mut(server.state.chest) {
            if server.state.tick == 0 {
                rotate_items(inv);
            }
        }

        server.clients.retain(|_, client| {
            if client.created_this_tick() {
                if self
                    .player_count
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                        (count < MAX_PLAYERS).then_some(count + 1)
                    })
                    .is_err()
                {
                    client.disconnect("The server is full!".color(Color::RED));
                    return false;
                }

                match server
                    .entities
                    .insert_with_uuid(EntityKind::Player, client.uuid(), ())
                {
                    Some((id, _)) => client.state.entity_id = id,
                    None => {
                        client.disconnect("Conflicting UUID");
                        return false;
                    }
                }

                client.spawn(world_id);
                client.set_flat(true);
                client.teleport(spawn_pos, 0.0, 0.0);
                client.set_player_list(server.state.player_list.clone());

                if let Some(id) = &server.state.player_list {
                    server.player_lists.get_mut(id).insert(
                        client.uuid(),
                        client.username(),
                        client.textures().cloned(),
                        client.game_mode(),
                        0,
                        None,
                    );
                }

                client.send_message("Welcome to Valence! Sneak to give yourself an item.".italic());
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                server.entities.remove(client.state.entity_id);
                if let Some(id) = &server.state.player_list {
                    server.player_lists.get_mut(id).remove(client.uuid());
                }
                return false;
            }

            let player = server.entities.get_mut(client.state.entity_id).unwrap();

            if client.position().y <= -20.0 {
                client.teleport(spawn_pos, client.yaw(), client.pitch());
            }

            while let Some(event) = handle_event_default(client, player) {
                match event {
                    ClientEvent::InteractWithBlock { hand, location, .. } => {
                        if hand == Hand::Main
                            && world.chunks.get_block_state(location) == Some(BlockState::CHEST)
                        {
                            client.send_message("Opening chest!");
                            let window = WindowInventory::new(1, server.state.chest);
                            client.send_packet(OpenScreen {
                                window_id: VarInt(window.window_id.into()),
                                window_type: VarInt(2),
                                window_title: "Extra".italic()
                                    + " Chesty".not_italic().bold().color(Color::RED)
                                    + " Chest".not_italic(),
                            });
                            client.open_inventory = Some(window);
                        }
                    }
                    ClientEvent::CloseScreen { window_id } => {
                        if window_id > 0 {
                            client.send_message(format!("Window closed: {}", window_id));
                            client.send_message(format!("Chest: {:?}", server.state.chest));
                        }
                    }
                    ClientEvent::ClickContainer {
                        window_id,
                        state_id,
                        slot_id,
                        mode,
                        slot_changes,
                        carried_item,
                    } => {
                        println!(
                            "window_id: {:?}, state_id: {:?}, slot_id: {:?}, mode: {:?}, \
                             slot_changes: {:?}, carried_item: {:?}",
                            window_id, state_id, slot_id, mode, slot_changes, carried_item
                        );
                        client.cursor_held_item = carried_item;
                        if let Some(window) = client.open_inventory.as_mut() {
                            if let Some(obj_inv) =
                                server.inventories.get_mut(window.object_inventory)
                            {
                                for (slot_id, slot) in slot_changes {
                                    if slot_id < obj_inv.slot_count() as SlotId {
                                        obj_inv.set_slot(slot_id, slot);
                                    } else {
                                        let offset = obj_inv.slot_count() as SlotId;
                                        client.inventory.set_slot(
                                            slot_id - offset + PlayerInventory::GENERAL_SLOTS.start,
                                            slot,
                                        );
                                    }
                                }
                            }
                        }
                    }
                    ClientEvent::StartSneaking => {
                        let slot_id: SlotId = PlayerInventory::HOTBAR_SLOTS.start;
                        let stack = match client.inventory.get_slot(slot_id) {
                            Slot::Empty => ItemStack {
                                item_count: 1,
                                item_id: VarInt(1),
                                nbt: None,
                            },
                            Slot::Present(s) => ItemStack {
                                item_count: s.item_count + 1,
                                item_id: s.item_id,
                                nbt: None,
                            },
                        };
                        client.inventory.set_slot(slot_id, Slot::Present(stack));
                    }
                    _ => {}
                }
            }

            true
        });
    }
}

fn rotate_items(inv: &mut ConfigurableInventory) {
    for i in 1..inv.slot_count() {
        let a = inv.get_slot((i - 1) as SlotId);
        let b = inv.get_slot(i as SlotId);
        inv.set_slot((i - 1) as SlotId, b);
        inv.set_slot(i as SlotId, a);
    }
}
