use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use valence::block::BlockState;
use valence::client::GameMode;
use valence::config::{Config, ServerListPing};
use valence::text::Color;
use valence::{
    async_trait, ChunkPos, ClientMut, DimensionId, EntityType, Server, ShutdownResult, Text,
    TextFormat, WorldId, WorldsMut,
};

pub fn main() -> ShutdownResult {
    env_logger::Builder::new()
        .filter_module("valence", LevelFilter::Trace)
        .parse_default_env()
        .init();

    valence::start_server(Game {
        player_count: AtomicUsize::new(0),
    })
}

struct Game {
    player_count: AtomicUsize,
}

const MAX_PLAYERS: usize = 10;

#[async_trait]
impl Config for Game {
    fn max_connections(&self) -> usize {
        // We want status pings to be successful even if the server is full.
        MAX_PLAYERS + 64
    }

    fn online_mode(&self) -> bool {
        // You'll want this to be true on real servers.
        false
    }

    async fn server_list_ping(&self, _server: &Server, _remote_addr: SocketAddr) -> ServerListPing {
        ServerListPing::Respond {
            online_players: self.player_count.load(Ordering::SeqCst) as i32,
            max_players: MAX_PLAYERS as i32,
            description: "Hello Valence!".color(Color::AQUA),
            favicon_png: Some(include_bytes!("favicon.png")),
        }
    }

    fn join(
        &self,
        _server: &Server,
        _client: ClientMut,
        worlds: WorldsMut,
    ) -> Result<WorldId, Text> {
        if let Ok(_) = self
            .player_count
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                (count < MAX_PLAYERS).then(|| count + 1)
            })
        {
            Ok(worlds.iter().next().unwrap().0)
        } else {
            Err("The server is full!".into())
        }
    }

    fn init(&self, _server: &Server, mut worlds: WorldsMut) {
        let world_id = worlds.create(DimensionId::default());
        let mut world = worlds.get_mut(world_id).unwrap();

        let size = 5;
        for z in -size..size {
            for x in -size..size {
                let pos = ChunkPos::new(x, z);
                world.chunks.create(pos);
                let mut chunk = world.chunks.get_mut(pos).unwrap();

                // Chunks are only visible to clients if all adjacent chunks are loaded.
                // This will make the perimiter chunks contain only air.
                if x != -size && x != size - 1 && z != -size && z != size - 1 {
                    for z in 0..16 {
                        for x in 0..16 {
                            let block_x = pos.x * 16 + x as i32;
                            let block_z = pos.z * 16 + z as i32;

                            let height = 50.0
                                + ((block_x as f64 / 10.0).cos() + (block_z as f64 / 10.0).sin())
                                    * 7.0;

                            for y in 0..height.round() as usize {
                                let states = [
                                    BlockState::ACACIA_PLANKS,
                                    BlockState::SLIME_BLOCK,
                                    BlockState::IRON_BLOCK,
                                    BlockState::SEA_LANTERN,
                                    BlockState::STONE,
                                    BlockState::DIRT,
                                    BlockState::PRISMARINE_BRICKS,
                                    BlockState::DIAMOND_ORE,
                                ];

                                chunk.set_block_state(x, y, z, states[y % states.len()]);
                            }
                        }
                    }
                }
            }
        }
    }

    fn update(&self, server: &Server, mut worlds: WorldsMut) {
        let mut world = worlds.iter_mut().next().unwrap().1;

        world.clients.retain(|_, mut client| {
            if client.created_tick() == server.current_tick() {
                client.set_game_mode(GameMode::Creative);
                client.teleport([0.0, 200.0, 0.0], 0.0, 0.0);
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                false
            } else {
                true
            }
        });
    }
}
