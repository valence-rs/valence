extern crate valence;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};

use valence::async_trait;
use valence::biome::Biome;
use valence::chunk::{Chunk, ChunkPos, UnloadedChunk};
use valence::client::{handle_event_default, GameMode};
use valence::config::{Config, ServerListPing};
use valence::dimension::DimensionId;
use valence::entity::{EntityId, EntityKind};
use valence::player_list::PlayerListId;
use valence::server::{Server, SharedServer, ShutdownResult};
use valence::text::{Color, TextFormat};
use valence::util::chunks_in_view_distance;
use valence_anvil::biome::BiomeKind;
use valence_anvil::AnvilWorld;

pub fn main() -> ShutdownResult {
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

const MAX_PLAYERS: usize = 10;

/// # IMPORTANT
/// Change the following to the world file you wish to load.
/// Inside this folder you should see `advancements`, `DIM1`, `DIM-1` and most
/// importantly `region` directories. Only the `region` directory is accessed.
const WORLD_FOLDER: &str = "./test_data/";

#[async_trait]
impl Config for Game {
    type ServerState = Option<PlayerListId>;
    type ClientState = EntityId;
    type EntityState = ();
    type WorldState = AnvilWorld;
    /// If the chunk should stay loaded at the end of the tick.
    type ChunkState = bool;
    type PlayerListState = ();

    fn max_connections(&self) -> usize {
        // We want status pings to be successful even if the server is full.
        MAX_PLAYERS + 64
    }

    fn biomes(&self) -> Vec<Biome> {
        BiomeKind::ALL.iter().map(|b| b.biome().unwrap()).collect()
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
            favicon_png: Some(
                include_bytes!("../../assets/logo-64x64.png")
                    .as_slice()
                    .into(),
            ),
        }
    }

    fn init(&self, server: &mut Server<Self>) {
        let world_folder = PathBuf::from_str(WORLD_FOLDER).unwrap();
        server.worlds.insert(
            DimensionId::default(),
            AnvilWorld::new(world_folder, &server.shared),
        );
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
                client.set_game_mode(GameMode::Spectator);
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

                client.send_message("Welcome to the terrain example!".italic());
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

            let required_chunks = chunks_in_view_distance(ChunkPos::at(p.x, p.z), dist);
            let mut new_chunks = Vec::new();
            for pos in required_chunks {
                if let Some(existing) = world.chunks.get_mut(pos) {
                    existing.state = true;
                } else {
                    new_chunks.push(pos);
                }
            }

            let future = world.state.load_chunks(new_chunks.into_iter());

            let parsed_chunks = futures::executor::block_on(future).unwrap();
            for (pos, chunk) in parsed_chunks {
                if let Some(chunk) = chunk {
                    world.chunks.insert(pos, chunk, true);
                } else {
                    let mut blank_chunk = UnloadedChunk::new(16);
                    blank_chunk.set_block_state(
                        0,
                        0,
                        0,
                        valence::block::BlockState::from_kind(valence::block::BlockKind::Lava),
                    );
                    world.chunks.insert(pos, blank_chunk, true);
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
    }
}
