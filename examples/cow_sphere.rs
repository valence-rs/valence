use std::f64::consts::TAU;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use valence::async_trait;
use valence::block::{BlockPos, BlockState};
use valence::client::GameMode;
use valence::config::{Config, ServerListPing};
use valence::dimension::DimensionId;
use valence::entity::{EntityId, EntityKind};
use valence::server::{Server, SharedServer, ShutdownResult};
use valence::text::{Color, TextFormat};
use valence::util::to_yaw_and_pitch;
use vek::{Mat3, Vec3};

pub fn main() -> ShutdownResult {
    env_logger::Builder::new()
        .filter_module("valence", LevelFilter::Trace)
        .parse_default_env()
        .init();

    valence::start_server(
        Game {
            player_count: AtomicUsize::new(0),
        },
        ServerData { cows: Vec::new() },
    )
}

struct Game {
    player_count: AtomicUsize,
}

struct ServerData {
    cows: Vec<EntityId>,
}

const MAX_PLAYERS: usize = 10;

const SPAWN_POS: BlockPos = BlockPos::new(0, 100, -25);

#[async_trait]
impl Config for Game {
    type ChunkData = ();
    type ClientData = ();
    type EntityData = ();
    type ServerData = ServerData;
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

        server.data.cows.extend((0..200).map(|_| {
            let (id, e) = server.entities.create(EntityKind::Cow, ());
            e.set_world(world_id);
            id
        }));
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
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                world.meta.player_list_mut().remove(client.uuid());
                return false;
            }

            true
        });

        let time = server.shared.current_tick() as f64 / server.shared.tick_rate() as f64;

        let rot = Mat3::rotation_x(time * TAU * 0.1)
            .rotated_y(time * TAU * 0.2)
            .rotated_z(time * TAU * 0.3);

        let radius = 6.0 + ((time * TAU / 2.5).sin() + 1.0) / 2.0 * 10.0;

        let player_pos = server
            .clients
            .iter()
            .next()
            .map(|c| c.1.position())
            .unwrap_or_default();

        // TODO: hardcoded eye pos.
        let eye_pos = Vec3::new(player_pos.x, player_pos.y + 1.6, player_pos.z);

        for (cow_id, p) in server
            .data
            .cows
            .iter()
            .cloned()
            .zip(fibonacci_spiral(server.data.cows.len()))
        {
            let cow = server.entities.get_mut(cow_id).expect("missing cow");
            let rotated = p * rot;
            let transformed = rotated * radius + [0.5, SPAWN_POS.y as f64 + 2.0, 0.5];

            let yaw = f32::atan2(rotated.z as f32, rotated.x as f32).to_degrees() - 90.0;
            let (looking_yaw, looking_pitch) =
                to_yaw_and_pitch((eye_pos - transformed).normalized());

            cow.set_position(transformed);
            cow.set_yaw(yaw);
            cow.set_pitch(looking_pitch as f32);
            cow.set_head_yaw(looking_yaw as f32);
        }
    }
}

/// Distributes N points on the surface of a unit sphere.
fn fibonacci_spiral(n: usize) -> impl Iterator<Item = Vec3<f64>> {
    (0..n).map(move |i| {
        let golden_ratio = (1.0 + 5_f64.sqrt()) / 2.0;

        // Map to unit square
        let x = i as f64 / golden_ratio % 1.0;
        let y = i as f64 / n as f64;

        // Map from unit square to unit sphere.
        let theta = x * TAU;
        let phi = (1.0 - 2.0 * y).acos();
        Vec3::new(theta.cos() * phi.sin(), theta.sin() * phi.sin(), phi.cos())
    })
}
