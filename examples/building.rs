use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use valence::prelude::*;

pub fn main() -> ShutdownResult {
    tracing_subscriber::fmt().init();

    valence::start_server(
        Game {
            player_count: AtomicUsize::new(0),
        },
        ServerState { player_list: None },
    )
}

struct Game {
    player_count: AtomicUsize,
}

struct ServerState {
    player_list: Option<PlayerListId>,
}

#[derive(Default)]
struct ClientState {
    entity_id: EntityId,
}

const MAX_PLAYERS: usize = 10;

const SIZE_X: i32 = 100;
const SIZE_Z: i32 = 100;

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
        for z in 0..SIZE_Z {
            for x in 0..SIZE_X {
                world
                    .chunks
                    .set_block_state([x, 0, z], BlockState::GRASS_BLOCK);
            }
        }
    }

    fn update(&self, server: &mut Server<Self>) {
        let (world_id, world) = server.worlds.iter_mut().next().unwrap();

        let spawn_pos = [SIZE_X as f64 / 2.0, 1.0, SIZE_Z as f64 / 2.0];

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
                        client.entity_id = id
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
                        true,
                    );
                }

                client.set_game_mode(GameMode::Creative);
                client.send_message("Welcome to Valence! Build something cool.".italic());
            }

            let player = server.entities.get_mut(client.entity_id).unwrap();

            while let Some(event) = client.next_event() {
                event.handle_default(client, player);
                match event {
                    ClientEvent::StartDigging { position, .. } => {
                        // Allows clients in creative mode to break blocks.
                        if client.game_mode() == GameMode::Creative {
                            world.chunks.set_block_state(position, BlockState::AIR);
                        }
                    }
                    ClientEvent::FinishDigging { position, .. } => {
                        // Allows clients in survival mode to break blocks.
                        world.chunks.set_block_state(position, BlockState::AIR);
                    }
                    ClientEvent::UseItemOnBlock { .. } => {
                        // TODO: reimplement when inventories are re-added.
                        /*
                        if hand == Hand::Main {
                            if let Some(stack) = client.held_item() {
                                if let Some(held_block_kind) = stack.item.to_block_kind() {
                                    let block_to_place = BlockState::from_kind(held_block_kind);

                                    if client.game_mode() == GameMode::Creative
                                        || client.consume_held_item(1).is_ok()
                                    {
                                        if world
                                            .chunks
                                            .block_state(position)
                                            .map(|s| s.is_replaceable())
                                            .unwrap_or(false)
                                        {
                                            world.chunks.set_block_state(position, block_to_place);
                                        } else {
                                            let place_at = position.get_in_direction(face);
                                            world.chunks.set_block_state(place_at, block_to_place);
                                        }
                                    }
                                }
                            }
                        }
                         */
                    }
                    _ => {}
                }
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                player.set_deleted(true);
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
