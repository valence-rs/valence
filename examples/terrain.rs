use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use noise::{NoiseFn, Seedable, SuperSimplex};
use rayon::iter::ParallelIterator;
use valence::block::{BlockState, PropName, PropValue};
use valence::client::GameMode;
use valence::config::{Config, ServerListPing};
use valence::text::Color;
use valence::util::chunks_in_view_distance;
use valence::{
    async_trait, ChunkPos, DimensionId, Server, SharedServer, ShutdownResult, TextFormat,
};
use vek::Lerp;

pub fn main() -> ShutdownResult {
    env_logger::Builder::new()
        .filter_module("valence", LevelFilter::Trace)
        .parse_default_env()
        .init();

    let seed = rand::random();

    valence::start_server(Game {
        player_count: AtomicUsize::new(0),
        density_noise: SuperSimplex::new().set_seed(seed),
        hilly_noise: SuperSimplex::new().set_seed(seed.wrapping_add(1)),
        stone_noise: SuperSimplex::new().set_seed(seed.wrapping_add(2)),
        gravel_noise: SuperSimplex::new().set_seed(seed.wrapping_add(3)),
        grass_noise: SuperSimplex::new().set_seed(seed.wrapping_add(4)),
    })
}

struct Game {
    player_count: AtomicUsize,
    density_noise: SuperSimplex,
    hilly_noise: SuperSimplex,
    stone_noise: SuperSimplex,
    gravel_noise: SuperSimplex,
    grass_noise: SuperSimplex,
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

    fn init(&self, server: &mut Server) {
        let (_, world) = server.worlds.create(DimensionId::default());
        world.meta.set_flat(true);
    }

    fn update(&self, server: &mut Server) {
        let (world_id, world) = server.worlds.iter_mut().next().unwrap();

        let mut chunks_to_unload = HashSet::<_>::from_iter(world.chunks.iter().map(|t| t.0));

        server.clients.retain(|_, client| {
            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                return false;
            }

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

                client.set_world(world_id);
                client.set_game_mode(GameMode::Creative);
                client.set_max_view_distance(32);
                client.teleport([0.0, 200.0, 0.0], 0.0, 0.0);

                world.meta.player_list_mut().insert(
                    client.uuid(),
                    client.username().to_string(),
                    client.textures().cloned(),
                    client.game_mode(),
                    0,
                    None,
                );

                client.send_message("Welcome to the terrain example!".italic());
                client.send_message(
                    "This demonstrates how to create infinite procedurally generated terrain."
                        .italic(),
                );
            }

            let dist = client.view_distance();
            let p = client.position();

            for pos in chunks_in_view_distance(ChunkPos::at(p.x, p.z), dist) {
                chunks_to_unload.remove(&pos);
                if world.chunks.get(pos).is_none() {
                    world.chunks.create(pos);
                }
            }

            true
        });

        for pos in chunks_to_unload {
            world.chunks.delete(pos);
        }

        world.chunks.par_iter_mut().for_each(|(pos, chunk)| {
            if chunk.created_tick() != server.shared.current_tick() {
                return;
            }

            for z in 0..16 {
                for x in 0..16 {
                    let block_x = x as i64 + pos.x as i64 * 16;
                    let block_z = z as i64 + pos.z as i64 * 16;

                    let mut in_terrain = false;
                    let mut depth = 0;

                    for y in (0..chunk.height()).rev() {
                        let b = terrain_column(
                            self,
                            block_x,
                            y as i64,
                            block_z,
                            &mut in_terrain,
                            &mut depth,
                        );
                        chunk.set_block_state(x, y, z, b);
                    }

                    // Add grass
                    for y in (0..chunk.height()).rev() {
                        if chunk.get_block_state(x, y, z).is_air()
                            && chunk.get_block_state(x, y - 1, z) == BlockState::GRASS_BLOCK
                        {
                            let density = fbm(
                                &self.grass_noise,
                                [block_x, y as i64, block_z].map(|a| a as f64 / 5.0),
                                4,
                                2.0,
                                0.7,
                            );

                            if density > 0.55 {
                                if density > 0.7 && chunk.get_block_state(x, y + 1, z).is_air() {
                                    let upper = BlockState::TALL_GRASS
                                        .set(PropName::Half, PropValue::Upper);
                                    let lower = BlockState::TALL_GRASS
                                        .set(PropName::Half, PropValue::Lower);

                                    chunk.set_block_state(x, y + 1, z, upper);
                                    chunk.set_block_state(x, y, z, lower);
                                } else {
                                    chunk.set_block_state(x, y, z, BlockState::GRASS);
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}

fn terrain_column(
    g: &Game,
    x: i64,
    y: i64,
    z: i64,
    in_terrain: &mut bool,
    depth: &mut u32,
) -> BlockState {
    const WATER_HEIGHT: i64 = 55;

    if has_terrain_at(g, x, y, z) {
        let gravel_height = WATER_HEIGHT
            - 1
            - (fbm(
                &g.gravel_noise,
                [x, y, z].map(|a| a as f64 / 10.0),
                3,
                2.0,
                0.5,
            ) * 6.0)
                .floor() as i64;

        if *in_terrain {
            if *depth > 0 {
                *depth -= 1;
                if y < gravel_height {
                    BlockState::GRAVEL
                } else {
                    BlockState::DIRT
                }
            } else {
                BlockState::STONE
            }
        } else {
            *in_terrain = true;
            let n = noise01(&g.stone_noise, [x, y, z].map(|a| a as f64 / 15.0));

            *depth = (n * 5.0).round() as u32;

            if y < gravel_height {
                BlockState::GRAVEL
            } else if y < WATER_HEIGHT - 1 {
                BlockState::DIRT
            } else {
                BlockState::GRASS_BLOCK
            }
        }
    } else {
        *in_terrain = false;
        *depth = 0;
        if y < WATER_HEIGHT {
            BlockState::WATER
        } else {
            BlockState::AIR
        }
    }
}

fn has_terrain_at(g: &Game, x: i64, y: i64, z: i64) -> bool {
    let hilly = Lerp::lerp_unclamped(
        0.1,
        1.0,
        noise01(&g.hilly_noise, [x, y, z].map(|a| a as f64 / 400.0)).powi(2),
    );

    let lower = 15.0 + 100.0 * hilly;
    let upper = lower + 100.0 * hilly;

    if y as f64 <= lower {
        return true;
    } else if y as f64 >= upper {
        return false;
    }

    let density = 1.0 - lerpstep(lower, upper, y as f64);

    let n = fbm(
        &g.density_noise,
        [x, y, z].map(|a| a as f64 / 100.0),
        4,
        2.0,
        0.5,
    );
    n < density
}

fn lerpstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    if x <= edge0 {
        0.0
    } else if x >= edge1 {
        1.0
    } else {
        (x - edge0) / (edge1 - edge0)
    }
}

fn fbm(noise: &SuperSimplex, p: [f64; 3], octaves: u32, lacunarity: f64, persistence: f64) -> f64 {
    let mut freq = 1.0;
    let mut amp = 1.0;
    let mut amp_sum = 0.0;
    let mut sum = 0.0;

    for _ in 0..octaves {
        let n = noise01(noise, p.map(|a| a * freq));
        sum += n * amp;
        amp_sum += amp;

        freq *= lacunarity;
        amp *= persistence;
    }

    // Scale the output to [0, 1]
    sum / amp_sum
}

fn noise01(noise: &SuperSimplex, xyz: [f64; 3]) -> f64 {
    (noise.get(xyz) + 1.0) / 2.0
}
