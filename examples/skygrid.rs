use std::net::SocketAddr;
use std::ops::Range;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use once_cell::sync::Lazy;
use rand::seq::SliceRandom;
use rayon::prelude::ParallelIterator;
use valence::prelude::*;

const MAX_PLAYERS: usize = 10;
const Y_RANGE: Range<i64> = -64..319;
const BLOCK_SPACING: i64 = 4;

static BLOCK_TYPES: Lazy<Vec<BlockState>> = Lazy::new(|| {
    BlockKind::ALL
        .into_iter()
        .map(|k| k.to_state())
        .filter(|b| b.is_opaque())
        .collect()
});

pub fn main() -> ShutdownResult {
    env_logger::Builder::new()
        .filter_module("valence", LevelFilter::Trace)
        .parse_default_env()
        .init();
    valence::start_server(
        Game {
            player_count: AtomicUsize::new(0),
        },
        None,
    )
}

struct Game {
    player_count: AtomicUsize,
}

#[async_trait]
impl Config for Game {
    type ServerState = Option<PlayerListId>;
    type ClientState = EntityId;
    type EntityState = ();
    type WorldState = ();
    /// If the chunk should stay loaded at the end of the tick.
    type ChunkState = bool;
    type PlayerListState = ();

    fn max_connections(&self) -> usize {
        // We want status pings to be successful even if the server is full.
        MAX_PLAYERS + 64
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
        server.worlds.insert(DimensionId::default(), ());
        server.state = Some(server.player_lists.insert(()).0);
    }

    fn update(&self, server: &mut Server<Self>) {
        let (world_id, world) = server.worlds.iter_mut().next().unwrap();

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
                    Some((id, _)) => client.state = id,
                    None => {
                        client.disconnect("Conflicting UUID");
                        return false;
                    }
                }

                client.spawn(world_id);
                client.set_flat(true);
                client.set_game_mode(GameMode::Creative);
                client.teleport([0.0, 200.0, 0.0], 0.0, 0.0);
                client.set_player_list(server.state.clone());

                if let Some(id) = &server.state {
                    server.player_lists.get_mut(id).insert(
                        client.uuid(),
                        client.username(),
                        client.textures().cloned(),
                        client.game_mode(),
                        0,
                        None,
                    );
                }

                client.send_message("Welcome to SkyGrid!".italic());
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                if let Some(id) = &server.state {
                    server.player_lists.get_mut(id).remove(client.uuid());
                }
                server.entities.remove(client.state);

                return false;
            }

            if let Some(entity) = server.entities.get_mut(client.state) {
                while handle_event_default(client, entity).is_some() {}
            }

            let dist = client.view_distance();
            let p = client.position();

            for pos in chunks_in_view_distance(ChunkPos::at(p.x, p.z), dist) {
                if let Some(chunk) = world.chunks.get_mut(pos) {
                    chunk.state = true;
                } else {
                    world.chunks.insert(pos, UnloadedChunk::default(), true);
                }
            }

            true
        });

        // Remove chunks outside the view distance of players.
        world.chunks.retain(|_, chunk| {
            if chunk.state {
                chunk.state = false;
                true
            } else {
                false
            }
        });

        // Generate chunk data for chunks created this tick.
        world.chunks.par_iter_mut().for_each(|(pos, chunk)| {
            if !chunk.created_this_tick() {
                return;
            }

            for z in 0..16 {
                for x in 0..16 {
                    let block_x = x as i64 + pos.x as i64 * 16;
                    let block_z = z as i64 + pos.z as i64 * 16;

                    for y in (0..chunk.height()).rev() {
                        let b = terrain_column(block_x, y as i64, block_z);
                        chunk.set_block_state(x, y, z, b);
                    }
                }
            }
        });
    }
}

fn terrain_column(x: i64, y: i64, z: i64) -> BlockState {
    if has_terrain_at(x, y, z) {
        *BLOCK_TYPES
            .choose(&mut rand::thread_rng())
            .unwrap_or(&BlockState::STONE)
    } else {
        BlockState::AIR
    }
}

fn has_terrain_at(x: i64, y: i64, z: i64) -> bool {
    Y_RANGE.min().unwrap_or(-64) <= y
        && y <= Y_RANGE.max().unwrap_or(319)
        && x % BLOCK_SPACING == 0
        && y % BLOCK_SPACING == 0
        && z % BLOCK_SPACING == 0
}
