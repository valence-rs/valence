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
        world.meta.set_flat(true);

        let size = 5;
        for z in -size..size {
            for x in -size..size {
                world.chunks.create([x, z]);
            }
        }

        let entity_id = world.entities.create();
        let mut entity = world.entities.get_mut(entity_id).unwrap();

        entity.set_type(EntityType::Cow);
        entity.set_position([0.0, 100.0, 0.0]);
        //entity.set_yaw(30.0);
        //entity.set_pitch(0.0);
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

        for (_, mut e) in world.entities.iter_mut() {
            let time = server.current_tick() as f64 / server.tick_rate() as f64;

            if e.typ() == EntityType::Cow {
                e.set_position(e.position() + [0.0, 0.0, 0.02]);
                let yaw = (time % 1.0 * 360.0) as f32;
                e.set_yaw(yaw);
                e.set_head_yaw(yaw);
            }
        }
    }
}
