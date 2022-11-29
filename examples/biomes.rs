use std::iter;
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
const BIOME_COUNT: usize = 10;
const MIN_Y: i32 = -64;

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

    fn biomes(&self) -> Vec<Biome> {
        (1..BIOME_COUNT)
            .map(|i| {
                let color = (0xffffff / BIOME_COUNT * i) as u32;
                Biome {
                    name: ident!("valence:test_biome_{i}"),
                    sky_color: color,
                    water_fog_color: color,
                    fog_color: color,
                    water_color: color,
                    foliage_color: Some(color),
                    grass_color: Some(color),
                    ..Default::default()
                }
            })
            .chain(iter::once(Biome {
                name: ident!("plains"),
                ..Default::default()
            }))
            .collect()
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

        let height = world.chunks.height();
        assert_eq!(world.chunks.min_y(), MIN_Y);

        for chunk_z in 0..3 {
            for chunk_x in 0..3 {
                let chunk = if chunk_x == 1 && chunk_z == 1 {
                    let mut chunk = UnloadedChunk::new(height);

                    // Set chunk blocks
                    for z in 0..16 {
                        for x in 0..16 {
                            chunk.set_block_state(x, 1, z, BlockState::GRASS_BLOCK);
                        }
                    }

                    // Set chunk biomes
                    for z in 0..4 {
                        for x in 0..4 {
                            for y in 0..height / 4 {
                                let biome_id = server
                                    .shared
                                    .biomes()
                                    .nth((x + z * 4 + y * 4 * 4) % BIOME_COUNT)
                                    .unwrap()
                                    .0;

                                chunk.set_biome(x, y, z, biome_id);
                            }
                        }
                    }

                    chunk
                } else {
                    UnloadedChunk::default()
                };

                world.chunks.insert([chunk_x, chunk_z], chunk, ());
            }
        }
    }

    fn update(&self, server: &mut Server<Self>) {
        let (world_id, _) = server.worlds.iter_mut().next().unwrap();

        let spawn_pos = [24.0, 50.0, 24.0];

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
                        client.state.entity_id = id
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
                    server.player_lists.get_mut(id).insert(
                        client.uuid(),
                        client.username(),
                        client.textures().cloned(),
                        client.game_mode(),
                        0,
                        None,
                    );
                }

                client.set_game_mode(GameMode::Creative);
            }

            while client.next_event().is_some() {}

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                server.entities.remove(client.state.entity_id);
                if let Some(id) = &server.state.player_list {
                    server.player_lists.get_mut(id).remove(client.uuid());
                }
                return false;
            }

            if client.position().y < MIN_Y as _ {
                client.teleport(spawn_pos, client.yaw(), client.pitch());
            }

            true
        });
    }
}
