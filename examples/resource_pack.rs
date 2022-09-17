use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use valence::async_trait;
use valence::block::{BlockPos, BlockState};
use valence::chunk::UnloadedChunk;
use valence::client::{
    handle_event_default, ClientEvent, GameMode, InteractWithEntityKind, ResourcePackStatus,
};
use valence::config::{Config, ServerListPing};
use valence::dimension::DimensionId;
use valence::entity::{EntityId, EntityKind, TrackedData};
use valence::player_list::PlayerListId;
use valence::server::{Server, SharedServer, ShutdownResult};
use valence::text::{Color, TextFormat};

pub fn main() -> ShutdownResult {
    env_logger::Builder::new()
        .filter_module("valence", LevelFilter::Trace)
        .parse_default_env()
        .init();

    valence::start_server(
        Game {
            player_count: AtomicUsize::new(0),
        },
        ServerState {
            player_list: None,
            sheep_id: None,
        },
    )
}

struct Game {
    player_count: AtomicUsize,
}

struct ServerState {
    player_list: Option<PlayerListId>,
    sheep_id: Option<EntityId>,
}

#[derive(Default)]
struct ClientState {
    entity_id: EntityId,
    resource_pack_active: bool,
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
        let (world_id, world) = server.worlds.insert(DimensionId::default(), ());
        server.state.player_list = Some(server.player_lists.insert(()).0);

        let size = 5;
        for z in -size..size {
            for x in -size..size {
                world.chunks.insert([x, z], UnloadedChunk::default(), ());
            }
        }

        let (sheep_id, sheep) = server.entities.insert(EntityKind::Sheep, ());
        server.state.sheep_id = Some(sheep_id);
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
                    Some((id, _)) => client.state.entity_id = id,
                    None => {
                        client.disconnect("Conflicting UUID");
                        return false;
                    }
                }

                client.spawn(world_id);
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
                    server.player_lists.get_mut(id).insert(
                        client.uuid(),
                        client.username(),
                        client.textures().cloned(),
                        client.game_mode(),
                        0,
                        None,
                    );
                }

                client.send_message(
                    "Hit the sheep above you to activate the resource pack.".italic(),
                );
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                if let Some(id) = &server.state.player_list {
                    server.player_lists.get_mut(id).remove(client.uuid());
                }
                server.entities.remove(client.state.entity_id);

                return false;
            }

            let player = server.entities.get_mut(client.state.entity_id).unwrap();

            while let Some(event) = handle_event_default(client, player) {
                match event {
                    ClientEvent::InteractWithEntity { kind, id, .. } => {
                        if kind == InteractWithEntityKind::Attack
                            && Some(id) == server.state.sheep_id
                        {
                            if client.state.resource_pack_active {
                                client.send_message("You already have the resource pack!".italic());
                                continue;
                            }

                            client.set_resource_pack(
                                "http://localhost/example_pack.zip".to_owned(),
                                None,
                                false,
                                None,
                            );
                        }
                    }
                    ClientEvent::ResourcePackStatusChanged(s) => {
                        if matches!(s, ResourcePackStatus::SuccessfullyLoaded) {
                            client.state.resource_pack_active = true;
                            client.send_message(
                                "Thank you for accepting the resource pack!".italic(),
                            );
                        }
                    }
                    _ => {}
                }
            }

            true
        });
    }
}
