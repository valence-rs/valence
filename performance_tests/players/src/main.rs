use std::net::SocketAddr;
use std::time::Instant;

use log::LevelFilter;
use valence::prelude::*;

pub fn main() -> ShutdownResult {
    env_logger::Builder::new()
        .filter_module("valence", LevelFilter::Trace)
        .parse_default_env()
        .init();

    valence::start_server(
        Game,
        ServerState {
            player_list: None,
            time: None,
            millis_sum: 0.0,
        },
    )
}

const WITH_PLAYER_ENTITIES: bool = true;
const MAX_PLAYERS: usize = 10_000;

struct Game;

struct ServerState {
    player_list: Option<PlayerListId>,
    time: Option<Instant>,
    millis_sum: f64,
}

#[async_trait]
impl Config for Game {
    type ServerState = ServerState;
    type ClientState = EntityId;
    type EntityState = ();
    type WorldState = ();
    type ChunkState = ();
    type PlayerListState = ();

    fn max_connections(&self) -> usize {
        MAX_PLAYERS + 64
    }

    fn connection_mode(&self) -> ConnectionMode {
        ConnectionMode::Offline
    }

    fn outgoing_capacity(&self) -> usize {
        usize::MAX
    }

    fn dimensions(&self) -> Vec<Dimension> {
        vec![Dimension {
            natural: false,
            ambient_light: 1.0,
            fixed_time: None,
            effects: Default::default(),
            min_y: 0,
            height: 256,
        }]
    }

    async fn server_list_ping(
        &self,
        _shared: &SharedServer<Self>,
        _remote_addr: SocketAddr,
        _protocol_version: i32,
    ) -> ServerListPing {
        ServerListPing::Respond {
            online_players: -1,
            max_players: -1,
            player_sample: Default::default(),
            description: "Test server!".into(),
            favicon_png: None,
        }
    }

    fn init(&self, server: &mut Server<Self>) {
        server.state.player_list = Some(server.player_lists.insert(()).0);

        let (_, world) = server.worlds.insert(DimensionId::default(), ());

        let size = 5;
        for chunk_z in -size..size {
            for chunk_x in -size..size {
                let mut chunk = UnloadedChunk::new(16);
                for z in 0..16 {
                    for x in 0..16 {
                        chunk.set_block_state(x, 0, z, BlockState::GRASS_BLOCK);
                    }
                }

                world.chunks.insert([chunk_x, chunk_z], chunk, ());
            }
        }
    }

    fn update(&self, server: &mut Server<Self>) {
        if let Some(time) = &mut server.state.time {
            let millis = time.elapsed().as_secs_f64() * 1000.0;
            let tick = server.shared.current_tick();
            let players = server.clients.len();
            let delay = 20;

            server.state.millis_sum += millis;

            if tick % delay == 0 {
                let avg = server.state.millis_sum / delay as f64;
                println!("Avg delta: {avg:.3}ms tick={tick} players={players}");
                server.state.millis_sum = 0.0;
            }
        }
        server.state.time = Some(Instant::now());

        let (world_id, _) = server.worlds.iter_mut().next().unwrap();

        server.clients.retain(|_, client| {
            if client.created_this_tick() {
                log::info!("Client \"{}\" joined", client.username());

                client.spawn(world_id);
                client.set_flat(true);
                client.teleport([0.0, 1.0, 0.0], 0.0, 0.0);

                if WITH_PLAYER_ENTITIES {
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

                    match server
                        .entities
                        .insert_with_uuid(EntityKind::Player, client.uuid(), ())
                    {
                        Some((id, _)) => client.state = id,
                        None => {
                            client.disconnect("Conflicting UUID");
                            return false;
                        }
                    }
                }
            }

            if client.is_disconnected() {
                log::info!("Client \"{}\" disconnected", client.username());

                if WITH_PLAYER_ENTITIES {
                    if let Some(id) = &server.state.player_list {
                        server.player_lists.get_mut(id).remove(client.uuid());
                    }
                    server.entities.remove(client.state);
                }

                return false;
            }

            if WITH_PLAYER_ENTITIES {
                if let Some(entity) = server.entities.get_mut(client.state) {
                    while handle_event_default(client, entity).is_some() {}
                }
            } else {
                while let Some(event) = client.pop_event() {
                    if let ClientEvent::SettingsChanged { view_distance, .. } = event {
                        client.set_view_distance(view_distance);
                    }
                }
            }

            true
        });
    }
}
