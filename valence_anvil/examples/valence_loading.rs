extern crate valence;

use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use valence::prelude::*;
// use valence_anvil::biome::BiomeKind;
use valence_anvil::AnvilWorld;

/// # IMPORTANT
///
/// Run this example with one argument containing the path of the the following
/// to the world directory you wish to load. Inside this directory you can
/// commonly see `advancements`, `DIM1`, `DIM-1` and most importantly `region`
/// subdirectories. Only the `region` directory is accessed.
pub fn main() -> ShutdownResult {
    let Some(world_dir) = env::args().nth(1) else {
        return Err("Please add the world directory as program argument.".into())
    };

    let world_dir = PathBuf::from(world_dir);

    if !world_dir.exists() || !world_dir.is_dir() {
        return Err("World argument must be a directory that exists".into())
    }

    if !world_dir.join("region").exists() {
        return Err("Could not find the \"region\" directory in the given world directory".into())
    }

    valence::start_server(
        Game {
            world_dir,
            player_count: AtomicUsize::new(0),
        },
        None,
    )
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

    // fn biomes(&self) -> Vec<Biome> {
    //     BiomeKind::ALL.iter().map(|b| b.biome().unwrap()).collect()
    // }

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
            server.worlds.insert(id, AnvilWorld::new(&self.world_dir));
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
                client.teleport([0.0, 125.0, 0.0], 0.0, 0.0);
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
                server.entities.delete(client.id);

                return false;
            }

            if let Some(entity) = server.entities.get_mut(client.state.id) {
                while let Some(event) = client.next_event() {
                    event.handle_default(client, entity);
                }
            }

            let dist = client.view_distance();
            let p = client.position();

            for pos in ChunkPos::at(p.x, p.z).in_view(dist) {
                if let Some(existing) = world.chunks.get_mut(pos) {
                    existing.state = true;
                } else {
                    match world.state.read_chunk(pos.x, pos.z) {
                        Ok(Some(anvil_chunk)) => {
                            let mut chunk = UnloadedChunk::new(24);

                            if let Err(e) = valence_anvil::to_valence(
                                &anvil_chunk.data,
                                &mut chunk,
                                4,
                                |_| BiomeId::default(),
                            ) {
                                eprintln!(
                                    "failed to convert chunk at ({}, {}): {e}",
                                    pos.x, pos.z
                                );
                            }

                            world.chunks.insert(pos, chunk, true);
                        }
                        Ok(None) => {
                            // No chunk at this position.
                            world.chunks.insert(pos, UnloadedChunk::default(), true);
                        }
                        Err(e) => {
                            eprintln!("failed to read chunk at ({}, {}): {e}", pos.x, pos.z)
                        }
                    }
                }
            }

            true
        });

        for (_, chunk) in world.chunks.iter_mut() {
            if !chunk.state {
                chunk.set_deleted(true)
            }
        }
    }
}
