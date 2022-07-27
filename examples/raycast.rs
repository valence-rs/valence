use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use valence::async_trait;
use valence::block::{BlockPos, BlockState};
use valence::client::GameMode;
use valence::config::{Config, ServerListPing};
use valence::dimension::DimensionId;
use valence::entity::{EntityEnum, EntityKind};
use valence::server::{Server, SharedServer, ShutdownResult};
use valence::spatial_index::RaycastHit;
use valence::text::{Color, TextFormat};
use valence::util::from_yaw_and_pitch;
use vek::Vec3;

pub fn main() -> ShutdownResult {
    env_logger::Builder::new()
        .filter_module("valence", LevelFilter::Trace)
        .parse_default_env()
        .init();

    valence::start_server(
        Game {
            player_count: AtomicUsize::new(0),
        },
        (),
    )
}

struct Game {
    player_count: AtomicUsize,
}

const MAX_PLAYERS: usize = 10;

const SPAWN_POS: BlockPos = BlockPos::new(0, 100, -5);

const PLAYER_EYE_HEIGHT: f64 = 1.6;

#[async_trait]
impl Config for Game {
    type ChunkData = ();
    type ClientData = ();
    /// `true` for entities that have been intersected with.
    type EntityData = bool;
    type ServerData = ();
    type WorldData = ();

    fn max_connections(&self) -> usize {
        // We want status pings to be successful even if the server is full.
        MAX_PLAYERS + 64
    }

    fn online_mode(&self) -> bool {
        // You'll want this to be true on real servers.
        false
    }

    async fn server_list_ping(
        &self,
        _server: &SharedServer<Self>,
        _remote_addr: SocketAddr,
    ) -> ServerListPing {
        ServerListPing::Respond {
            online_players: self.player_count.load(Ordering::SeqCst) as i32,
            max_players: MAX_PLAYERS as i32,
            description: "Hello Valence!".color(Color::AQUA),
            favicon_png: Some(include_bytes!("../assets/favicon.png")),
        }
    }

    fn init(&self, server: &mut Server<Self>) {
        let (world_id, world) = server.worlds.create(DimensionId::default(), ());
        world.meta.set_flat(true);

        let size = 5;
        for z in -size..size {
            for x in -size..size {
                world.chunks.create([x, z], ());
            }
        }

        world.chunks.set_block_state(SPAWN_POS, BlockState::BEDROCK);

        const SHEEP_COUNT: usize = 10;
        for i in 0..SHEEP_COUNT {
            let offset = (i as f64 - (SHEEP_COUNT - 1) as f64 / 2.0) * 1.25;

            let (_, sheep) = server.entities.create(EntityKind::Sheep, false);
            sheep.set_world(world_id);
            sheep.set_position([offset + 0.5, SPAWN_POS.y as f64 + 1.0, 0.0]);
            sheep.set_yaw(180.0);
            sheep.set_head_yaw(180.0);
        }
    }

    fn update(&self, server: &mut Server<Self>) {
        let (world_id, world) = server.worlds.iter_mut().next().unwrap();

        server.clients.retain(|_, client| {
            if client.created_tick() == server.shared.current_tick() {
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

                client.spawn(world_id);
                client.set_game_mode(GameMode::Creative);
                client.teleport(
                    [
                        SPAWN_POS.x as f64 + 0.5,
                        SPAWN_POS.y as f64 + 1.0,
                        SPAWN_POS.z as f64 + 0.5,
                    ],
                    0.0,
                    0.0,
                );

                world.meta.player_list_mut().insert(
                    client.uuid(),
                    client.username().to_owned(),
                    client.textures().cloned(),
                    client.game_mode(),
                    0,
                    None,
                );

                client.send_message(
                    "Look at a sheep to change its ".italic()
                        + "color".italic().color(Color::GREEN)
                        + ".",
                );
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                world.meta.player_list_mut().remove(client.uuid());
                return false;
            }

            let client_pos = client.position();

            let origin = Vec3::new(client_pos.x, client_pos.y + PLAYER_EYE_HEIGHT, client_pos.z);
            let direction = from_yaw_and_pitch(client.yaw() as f64, client.pitch() as f64);
            let only_sheep = |hit: &RaycastHit| {
                server
                    .entities
                    .get(hit.entity)
                    .map_or(false, |e| e.kind() == EntityKind::Sheep)
            };

            if let Some(hit) = world.spatial_index.raycast(origin, direction, only_sheep) {
                if let Some(e) = server.entities.get_mut(hit.entity) {
                    e.data = true;
                }
            }

            true
        });

        for (_, e) in server.entities.iter_mut() {
            let intersected = e.data;
            if let EntityEnum::Sheep(sheep) = &mut e.view_mut() {
                if intersected {
                    sheep.set_color(5);
                } else {
                    sheep.set_color(0);
                }
            }
            e.data = false;
        }
    }
}
