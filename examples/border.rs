use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use num::Integer;
use valence::biome::Biome;
use valence::block::BlockState;
use valence::chunk::UnloadedChunk;
use valence::client::{handle_event_default, ClientEvent};
use valence::config::{Config, ServerListPing};
use valence::dimension::{Dimension, DimensionId};
use valence::entity::{EntityId, EntityKind};
use valence::player_list::PlayerListId;
use valence::protocol::packets::s2c::play::{
    InitializeWorldBorder, SetBorderCenter, SetBorderLerpSize, SetBorderSize,
};
use valence::server::{Server, SharedServer, ShutdownResult};
use valence::text::{Color, TextFormat};
use valence::{async_trait, ident};

pub fn main() -> ShutdownResult {
    env_logger::Builder::new()
        .filter_module("valence", LevelFilter::Trace)
        .parse_default_env()
        .init();

    valence::start_server(
        Game {
            player_count: AtomicUsize::new(0),
        },
        Default::default(),
    )
}

struct Game {
    player_count: AtomicUsize,
}

#[derive(Default)]
struct ClientState {
    entity_id: EntityId,
    shrink: bool,
}

#[derive(Default)]
struct ServerState {
    player_list: Option<PlayerListId>,
}

const MAX_PLAYERS: usize = 10;

const SIZE_X: i32 = 100;
const SIZE_Z: i32 = 100;
const FLOOR_Y: i32 = 64;

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

    fn online_mode(&self) -> bool {
        false
    }

    fn dimensions(&self) -> Vec<Dimension> {
        vec![Dimension {
            fixed_time: Some(6000),
            ..Dimension::default()
        }]
    }

    fn biomes(&self) -> Vec<Biome> {
        vec![Biome {
            name: ident!("valence:default_biome"),
            grass_color: Some(0x00ff00),
            ..Biome::default()
        }]
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
            description: "Hello Valence!".color(Color::AQUA),
            favicon_png: Some(include_bytes!("../assets/logo-64x64.png").as_slice().into()),
            player_sample: Default::default(),
        }
    }

    fn init(&self, server: &mut Server<Self>) {
        let world = server.worlds.insert(DimensionId::default(), ()).1;
        server.state.player_list = Some(server.player_lists.insert(()).0);

        let chunks_needed_x = Integer::div_ceil(&(SIZE_X as i32), &16) + 2;
        let chunks_needed_z = Integer::div_ceil(&(SIZE_Z as i32), &16) + 2;

        for chunk_z in -chunks_needed_z..chunks_needed_z {
            for chunk_x in -chunks_needed_x..chunks_needed_x {
                world.chunks.insert(
                    [chunk_x as i32, chunk_z as i32],
                    UnloadedChunk::default(),
                    (),
                );
            }
        }

        for z in -SIZE_Z..SIZE_Z {
            for x in -SIZE_X..SIZE_X {
                world
                    .chunks
                    .set_block_state([x, FLOOR_Y, z], BlockState::END_STONE);
            }
        }
    }

    fn update(&self, server: &mut Server<Self>) {
        let (world_id, world) = server.worlds.iter_mut().next().unwrap();

        let spawn_pos = [10 as f64 / 2.0, FLOOR_Y as f64 + 1.0, 10 as f64 / 2.0];

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

                client.set_flat(true);
                client.spawn(world_id);
                client.teleport(spawn_pos, 0.0, 0.0);

                client.send_packet(InitializeWorldBorder {
                    x: 0.0,
                    z: 0.0,
                    old_diameter: 20.0,
                    new_diameter: 20.0,
                    speed: 3000.into(),
                    portal_teleport_boundary: 5.into(), // limits diameter: with teleport_boundary X, max diameter is 2*X
                    warning_blocks: 10.into(),
                    warning_time: 1.into(),
                });

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
                    "Sneak or dig to change the border. Type a number in chat to change the \
                     diameter.",
                );
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                server.entities.remove(client.state.entity_id);

                if let Some(list) = client.player_list() {
                    server.player_lists.get_mut(list).remove(client.uuid());
                }

                return false;
            }

            let player = server.entities.get_mut(client.state.entity_id).unwrap();

            while let Some(event) = handle_event_default(client, player) {
                match event {
                    ClientEvent::StartSneaking => {
                        if client.state.shrink {
                            client.set_world_border([0.0, 5.0], 5.0);
                        } else {
                            client.set_world_border([5.0, 0.0], 10.0);
                        }
                        client.state.shrink = !client.state.shrink;
                    }
                    ClientEvent::Digging { .. } => {
                        client.resize_world_border(1.0, 4.0, 2000);
                    }
                    ClientEvent::ChatMessage { message, timestamp } => {
                        println!("Got {message}");
                        if let Ok(m) = message.parse() {
                            println!("Sending {m}");
                            client.send_packet(SetBorderSize { diameter: m })
                        }
                    }
                    _ => {}
                }
            }

            true
        });
    }
}
