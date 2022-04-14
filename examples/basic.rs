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
        let id = server.worlds.create(DimensionId::default());
        let mut worlds = server.worlds.worlds_mut().unwrap();
        let world = server.worlds.get(&mut worlds, id).unwrap();

        let chunk_radius = 5;

        for z in -chunk_radius..chunk_radius {
            for x in -chunk_radius..chunk_radius {
                let id = server.chunks.create(384);
                let mut chunks = server.chunks.chunks_mut().unwrap();

                let chunk = server.chunks.get(&mut chunks, id).unwrap();

                for z in 0..16 {
                    for x in 0..16 {
                        for y in 0..50 {
                            chunk.set_block_state(x, y, z, 1);
                        }
                    }
                }

                world.chunks_mut().insert((x, z).into(), id);
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
        let mut clients = server.entities.clients_mut().unwrap();

        let world_id = server.worlds.ids().nth(0).unwrap();

        let mut to_remove = Vec::new();

        for (client_id, client) in server.entities.iter(&mut clients) {
            if let Some(client) = client.get_mut() {
                if client.created_tick() == server.current_tick() {
                    client.set_world(world_id);
                    client.teleport(glm::vec3(0.0, 200.0, 0.0), 0.0, 0.0);
                }
            }

            if client.is_disconnected() {
                to_remove.push(client_id);
            }
        }

        drop(clients);

        for id in to_remove {
            server.entities.delete(id);
        }
    }
}
