extern crate valence;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use valence::prelude::*;
use valence_anvil::biome::BiomeKind;
use valence_anvil::AnvilWorld;

/// # IMPORTANT
///
/// Run this example with one argument containing the path of the the following
/// to the world directory you wish to load. Inside this directory you can
/// commonly see `advancements`, `DIM1`, `DIM-1` and most importantly `region`
/// subdirectories. Only the `region` directory is accessed.
pub fn main() -> ShutdownResult {
    let args: Vec<String> = std::env::args().collect();
    if let Some(world_folder) = args.get(1) {
        let world_folder = PathBuf::from(world_folder);
        if world_folder.exists() && world_folder.is_dir() {
            if !world_folder.join("region").exists() {
                ShutdownResult::Err(
                    "Could not find the `region` folder inside the world directory.".into(),
                )
            } else {
                // This actually starts and runs the server.
                valence::start_server(
                    Game {
                        world_dir: world_folder,
                        player_count: AtomicUsize::new(0),
                    },
                    None,
                )
            }
        } else {
            ShutdownResult::Err(
                "World directory argument is not valid: Must be a folder that exists.".into(),
            )
        }
    } else {
        ShutdownResult::Err("Please add the world directory as program argument.".into())
    }
}

#[derive(Debug, Default)]
struct ClientData {
    id: EntityId,
    //block: valence::block::BlockKind
}

struct Game {
    world_dir: PathBuf,
    player_count: AtomicUsize,
}

const MAX_PLAYERS: usize = 10;

#[async_trait]
impl Config for Game {
    type ServerState = Option<PlayerListId>;
    type ClientState = ClientData;
    type EntityState = ();
    type WorldState = AnvilWorld;
    /// If the chunk should stay loaded at the end of the tick.
    type ChunkState = bool;
    type PlayerListState = ();
    type InventoryState = ();

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
        for (id, dimension) in server.shared.dimensions() {
            server.worlds.insert(
                id,
                AnvilWorld::new::<Game, _>(&dimension, &self.world_dir, server.shared.biomes()),
            );
        }
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
                    Some((id, _)) => client.state.id = id,
                    None => {
                        client.disconnect("Conflicting UUID");
                        return false;
                    }
                }

                client.respawn(world_id);
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

                client.send_message("Welcome to the java chunk parsing example!");
                client.send_message(
                    "Chunks with a single lava source block indicates that the chunk is not \
                     (fully) generated."
                        .italic(),
                );
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                if let Some(id) = &server.state {
                    server.player_lists.get_mut(id).remove(client.uuid());
                }
                server.entities.remove(client.state.id);

                return false;
            }

            if let Some(entity) = server.entities.get_mut(client.state.id) {
                while let Some(event) = client.next_event() {
                    event.handle_default(client, entity);
                }
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
                    blank_chunk.set_block_state(0, 0, 0, BlockState::from_kind(BlockKind::Lava));
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
