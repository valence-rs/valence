use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use num::Integer;
use valence::inventory::{GENERAL_SLOTS, HOTBAR_SLOTS};
use valence::prelude::*;

pub fn main() -> ShutdownResult {
    tracing_subscriber::fmt().init();

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
    /// The client's player entity.
    player: EntityId,
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
    type InventoryState = ();

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
                    [chunk_x, chunk_z],
                    UnloadedChunk::default(),
                    (),
                );
            }
        }

        // initialize blocks in the chunks
        for x in 0..SIZE_X {
            for z in 0..SIZE_Z {
                world
                    .chunks
                    .set_block_state((x as i32, 0, z as i32), BlockState::GRASS_BLOCK);
            }
        }

        world.chunks.set_block_state((50, 0, 54), BlockState::STONE);
        world.chunks.set_block_state((50, 1, 54), BlockState::CHEST);

        // create chest inventory
        let title = "Extra".italic()
            + " Chesty".not_italic().bold().color(Color::RED)
            + " Chest".not_italic();

        let (id, _inv) = server.inventories.insert(InventoryKind::Generic9x3, title, ());
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
                    Some((id, entity)) => {
                        entity.set_world(world_id);
                        client.state.player = id
                    }
                    None => {
                        client.disconnect("Conflicting UUID");
                        return false;
                    }
                }

                client.respawn(world_id);
                client.set_flat(true);
                client.teleport(spawn_pos, 0.0, 0.0);
                client.set_player_list(server.state.player_list.clone());

                if let Some(id) = &server.state.player_list {
                    server.player_lists[id].insert(
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

            let player = server.entities.get_mut(client.state.player).unwrap();

            while let Some(event) = client.next_event() {
                event.handle_default(client, player);
                match event {
                    ClientEvent::UseItemOnBlock { hand, position, .. } => {
                        if hand == Hand::Main
                            && world.chunks.block_state(position) == Some(BlockState::CHEST)
                        {
                            client.send_message("Opening chest!");
                            client.set_open_inventory(server.state.chest);
                        }
                    }
                    ClientEvent::CloseContainer { window_id } => {
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
                        ..
                    } => {
                        println!(
                            "window_id: {:?}, state_id: {:?}, slot_id: {:?}, mode: {:?}, \
                             slot_changes: {:?}, carried_item: {:?}",
                            window_id, state_id, slot_id, mode, slot_changes, carried_item
                        );
                        client.replace_cursor_item(carried_item);
                        if let Some(id) = client.open_inventory() {
                            if let Some(obj_inv) =
                                server.inventories.get_mut(id)
                            {
                                for (slot_id, slot) in slot_changes {
                                    let slot_id = slot_id as u16;
                                    if slot_id < obj_inv.slot_count() {
                                        obj_inv.replace_slot(slot_id, slot);
                                    } else {
                                        let offset = obj_inv.slot_count();
                                        let player_slot_id = slot_id - offset + GENERAL_SLOTS.start;
                                        client.replace_slot(player_slot_id, slot);
                                    }
                                }
                            }
                        }
                    }
                    ClientEvent::StartSneaking => {
                        let slot_id = HOTBAR_SLOTS.start;
                        let stack = match client.slot(slot_id) {
                            None => ItemStack::new(ItemKind::Stone, 1, None),
                            Some(s) => ItemStack::new(s.item, s.count() + 1, None),
                        };
                        client.replace_slot(slot_id, Some(stack));
                    }
                    _ => {}
                }
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                server.entities[client.player].set_deleted(true);
                if let Some(id) = &server.state.player_list {
                    server.player_lists[id].remove(client.uuid());
                }
                return false;
            }

            if client.position().y <= -20.0 {
                client.teleport(spawn_pos, client.yaw(), client.pitch());
            }

            true
        });
    }
}

fn rotate_items<C: Config>(inv: &mut Inventory<C>) {
    for i in 1..inv.slot_count() {
        inv.swap_slot(i - 1, i);
    }
}
