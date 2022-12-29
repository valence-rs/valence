use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use valence::prelude::*;
use valence_protocol::types::EntityInteraction;

pub fn main() -> ShutdownResult {
    tracing_subscriber::fmt().init();

    valence::start_server(
        Game {
            player_count: AtomicUsize::new(0),
        },
        ServerState {
            player_list: None,
            sheep_id: EntityId::NULL,
        },
    )
}

struct Game {
    player_count: AtomicUsize,
}

struct ServerState {
    player_list: Option<PlayerListId>,
    sheep_id: EntityId,
}

#[derive(Default)]
struct ClientState {
    entity_id: EntityId,
}

const MAX_PLAYERS: usize = 10;

const SPAWN_POS: BlockPos = BlockPos::new(0, 100, 0);

#[async_trait]
impl Config for Game {
    type ServerState = ServerState;
    type ClientState = ClientState;
    type EntityState = ();
    type WorldState = ();
    type ChunkState = ();
    type PlayerListState = ();
    type InventoryState = ();

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
        let (world_id, world) = server.worlds.insert(DimensionId::default(), ());
        server.state.player_list = Some(server.player_lists.insert(()).0);

        let size = 5;
        for z in -size..size {
            for x in -size..size {
                world.chunks.insert([x, z], UnloadedChunk::default(), ());
            }
        }

        let (sheep_id, sheep) = server.entities.insert(EntityKind::Sheep, ());
        server.state.sheep_id = sheep_id;
        sheep.set_world(world_id);
        sheep.set_position([
            SPAWN_POS.x as f64 + 0.5,
            SPAWN_POS.y as f64 + 4.0,
            SPAWN_POS.z as f64 + 0.5,
        ]);

        if let TrackedData::Sheep(sheep_data) = sheep.data_mut() {
            sheep_data.set_custom_name("Hit me".color(Color::GREEN));
        }

        world.chunks.set_block_state(SPAWN_POS, BlockState::BEDROCK);
    }

    fn update(&self, server: &mut Server<Self>) {
        let (world_id, _) = server.worlds.iter_mut().next().expect("missing world");

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
                    Some((id, _)) => client.entity_id = id,
                    None => {
                        client.disconnect("Conflicting UUID");
                        return false;
                    }
                }

                client.respawn(world_id);
                client.set_flat(true);
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
                client.set_player_list(server.state.player_list.clone());

                if let Some(id) = &server.state.player_list {
                    server.player_lists[id].insert(
                        client.uuid(),
                        client.username(),
                        client.textures().cloned(),
                        client.game_mode(),
                        0,
                        None,
                        true,
                    );
                }

                client.send_message(
                    "Hit the sheep above you to prompt for the resource pack again.".italic(),
                );

                set_example_pack(client);
            }

            let player = &mut server.entities[client.entity_id];

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                if let Some(id) = &server.state.player_list {
                    server.player_lists[id].remove(client.uuid());
                }
                player.set_deleted(true);

                return false;
            }

            while let Some(event) = client.next_event() {
                event.handle_default(client, player);
                match event {
                    ClientEvent::InteractWithEntity {
                        entity_id,
                        interact,
                        ..
                    } => {
                        if interact == EntityInteraction::Attack
                            && entity_id == server.state.sheep_id.to_raw()
                        {
                            set_example_pack(client);
                        }
                    }
                    ClientEvent::ResourcePackLoaded => {
                        client.send_message("Resource pack loaded!".color(Color::GREEN));
                    }
                    ClientEvent::ResourcePackDeclined => {
                        client.send_message("Resource pack declined.".color(Color::RED));
                    }
                    ClientEvent::ResourcePackFailedDownload => {
                        client.send_message("Resource pack download failed.".color(Color::RED));
                    }
                    _ => {}
                }
            }

            true
        });
    }
}

/// Sends the resource pack prompt.
fn set_example_pack(client: &mut Client<Game>) {
    client.set_resource_pack(
        "https://github.com/valence-rs/valence/raw/main/assets/example_pack.zip",
        "d7c6108849fb190ec2a49f2d38b7f1f897d9ce9f",
        false,
        None,
    );
}
