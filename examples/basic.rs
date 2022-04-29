use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use log::LevelFilter;
use valence::config::{Handler, ServerListPing};
use valence::text::Color;
use valence::{glm, DimensionId, Server, ServerConfig, SharedServer, ShutdownResult, TextFormat};

pub fn main() -> ShutdownResult {
    env_logger::Builder::new()
        .filter_level(LevelFilter::Trace)
        .parse_default_env()
        .init();

    let game = Game {
        favicon: Arc::from(include_bytes!("favicon.png").as_slice()),
    };

    let mut cfg = ServerConfig::new();

    cfg.handler(game);
    cfg.online_mode(false);
    cfg.start()
}

struct Game {
    favicon: Arc<[u8]>,
}

#[async_trait]
impl Handler for Game {
    fn init(&self, server: &mut Server) {
        let world_id = server.worlds.create(DimensionId::default());
        let world = server.worlds.get_mut(world_id).unwrap();

        let chunk_radius = 5;

        for z in -chunk_radius..chunk_radius {
            for x in -chunk_radius..chunk_radius {
                let chunk_id = server.chunks.create(384);
                let chunk = server.chunks.get_mut(chunk_id).unwrap();

                for z in 0..16 {
                    for x in 0..16 {
                        for y in 0..50 {
                            chunk.set_block_state(x, y, z, 1);
                        }
                    }
                }

                world.chunks_mut().insert((x, z).into(), chunk_id);
            }
        }
    }

    async fn server_list_ping(
        &self,
        server: &SharedServer,
        _remote_addr: SocketAddr,
    ) -> ServerListPing {
        ServerListPing::Respond {
            online_players: server.client_count() as i32,
            max_players: server.max_clients() as i32,
            description: "Hello Valence!".color(Color::AQUA),
            favicon_png: Some(self.favicon.clone()),
        }
    }

    fn update(&self, server: &mut Server) {
        let world_id = server.worlds.iter().next().unwrap().0;

        server.clients.retain(|_, client| {
            if client.created_tick() == server.other.current_tick() {
                client.set_world(Some(world_id));
                client.teleport(glm::vec3(0.0, 200.0, 0.0), 0.0, 0.0);
            }

            if client.is_disconnected() {
                server.entities.delete(client.entity());
                false
            } else {
                true
            }
        });
    }
}
