use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use valence::client::GameMode;
use valence::config::{Config, Login, ServerListPing};
use valence::text::Color;
use valence::{
    async_trait, DimensionId, NewClientData, Server, SharedServer, ShutdownResult, TextFormat,
};

pub fn main() -> ShutdownResult {
    env_logger::Builder::new()
        .filter_level(LevelFilter::Trace)
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

    fn init(&self, server: &mut Server) {
        let world_id = server.worlds.create(DimensionId::default());
        let world = server.worlds.get_mut(world_id).unwrap();

        let chunk_radius = 5;

        for z in -chunk_radius..chunk_radius {
            for x in -chunk_radius..chunk_radius {
                let chunk_id = server.chunks.create(384);
                let chunk = server.chunks.get_mut(chunk_id).unwrap();

                // Chunks are only visible to clients if all adjacent chunks are loaded.
                // This will make the perimiter chunks contain only air.
                if x != -chunk_radius
                    && x != chunk_radius - 1
                    && z != -chunk_radius
                    && z != chunk_radius - 1
                {
                    for z in 0..16 {
                        for x in 0..16 {
                            for y in 0..50 {
                                chunk.set_block_state(x, y, z, 1);
                            }
                        }
                    }
                }

                world.chunks_mut().insert((x, z).into(), chunk_id);
            }
        }
    }

    fn update(&self, server: &mut Server) {
        let world_id = server.worlds.iter().next().unwrap().0;

        server.clients.retain(|_, client| {
            if client.created_tick() == server.other.current_tick() {
                client.set_world(Some(world_id));
                client.set_game_mode(GameMode::Creative);
                client.teleport([0.0, 200.0, 0.0], 0.0, 0.0);
            }

            if client.is_disconnected() {
                server.entities.delete(client.entity());
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                false
            } else {
                true
            }
        });
    }

    async fn server_list_ping(
        &self,
        _server: &SharedServer,
        _remote_addr: SocketAddr,
    ) -> ServerListPing {
        ServerListPing::Respond {
            online_players: self.player_count.load(Ordering::SeqCst) as i32,
            max_players: MAX_PLAYERS as i32,
            description: "Hello Valence!".color(Color::AQUA),
            favicon_png: Some(include_bytes!("favicon.png")),
        }
    }

    async fn login(&self, _server: &SharedServer, _ncd: &NewClientData) -> Login {
        let res = self
            .player_count
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                (count < MAX_PLAYERS).then(|| count + 1)
            });

        if res.is_ok() {
            Login::Join
        } else {
            Login::Disconnect("The server is full!".into())
        }
    }
}
