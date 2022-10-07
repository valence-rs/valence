use std::mem;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use num::Integer;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use valence::biome::Biome;
use valence::block::BlockState;
use valence::chunk::{Chunk, UnloadedChunk};
use valence::client::{handle_event_default, ClientEvent, Hand};
use valence::config::{Config, ServerListPing};
use valence::dimension::{Dimension, DimensionId};
use valence::entity::types::Pose;
use valence::entity::{EntityId, EntityKind, TrackedData};
use valence::player_list::PlayerListId;
use valence::protocol::packets::s2c::play::SoundCategory;
use valence::server::{Server, SharedServer, ShutdownResult};
use valence::text::{Color, TextFormat};
use valence::{async_trait, ident};
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
        ServerState {
            player_list: None,
            paused: false,
            board: vec![false; SIZE_X * SIZE_Z].into_boxed_slice(),
            board_buf: vec![false; SIZE_X * SIZE_Z].into_boxed_slice(),
        },
    )
}

struct Game {
    player_count: AtomicUsize,
}

struct ServerState {
    player_list: Option<PlayerListId>,
    paused: bool,
    board: Box<[bool]>,
    board_buf: Box<[bool]>,
}

#[derive(Default)]
struct ClientState {
    entity_id: EntityId,
}

const MAX_PLAYERS: usize = 10;

const SIZE_X: usize = 100;
const SIZE_Z: usize = 100;
const BOARD_Y: i32 = 50;

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

    fn address(&self) -> SocketAddr {
        SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 25565).into() // TODO remove
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
            player_sample: Default::default(),
            description: "Hello Valence!".color(Color::AQUA),
            favicon_png: Some(include_bytes!("../assets/logo-64x64.png").as_slice().into()),
        }
    }

    fn init(&self, server: &mut Server<Self>) {
        let world = server.worlds.insert(DimensionId::default(), ()).1;
        server.state.player_list = Some(server.player_lists.insert(()).0);

        for chunk_z in -2..Integer::div_ceil(&(SIZE_Z as i32), &16) + 2 {
            for chunk_x in -2..Integer::div_ceil(&(SIZE_X as i32), &16) + 2 {
                world.chunks.insert(
                    [chunk_x as i32, chunk_z as i32],
                    UnloadedChunk::default(),
                    (),
                );
            }
        }
    }

    fn update(&self, server: &mut Server<Self>) {
        let (world_id, world) = server.worlds.iter_mut().next().unwrap();

        let spawn_pos = [
            SIZE_X as f64 / 2.0,
            BOARD_Y as f64 + 1.0,
            SIZE_Z as f64 / 2.0,
        ];

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
                client.teleport(spawn_pos, 0.0, 0.0);
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

                client.send_message("Welcome to Conway's game of life in Minecraft!".italic());
                client.send_message(
                    "Sneak and hold the left mouse button to bring blocks to life.".italic(),
                );
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                server.entities.remove(client.state.entity_id);
                if let Some(id) = &server.state.player_list {
                    server.player_lists.get_mut(id).remove(client.uuid());
                }
                return false;
            }

            let player = server.entities.get_mut(client.state.entity_id).unwrap();

            if client.position().y <= 0.0 {
                client.teleport(spawn_pos, client.yaw(), client.pitch());
                server.state.board.fill(false);
            }

            while let Some(event) = handle_event_default(client, player) {
                match event {
                    ClientEvent::Digging { position, .. } => {
                        if (0..SIZE_X as i32).contains(&position.x)
                            && (0..SIZE_Z as i32).contains(&position.z)
                            && position.y == BOARD_Y
                        {
                            let index = position.x as usize + position.z as usize * SIZE_X;

                            if !server.state.board[index] {
                                client.play_sound(
                                    ident!("minecraft:block.note_block.banjo"),
                                    SoundCategory::Block,
                                    Vec3::<i32>::from(position).as_(),
                                    0.5f32,
                                    1f32,
                                );
                            }

                            server.state.board[index] = true;
                        }
                    }
                    ClientEvent::InteractWithBlock { hand, .. } => {
                        if hand == Hand::Main {
                            client.send_message("I said left click, not right click!".italic());
                        }
                    }
                    _ => {}
                }
            }

            if let TrackedData::Player(data) = player.data() {
                let sneaking = data.get_pose() == Pose::Sneaking;
                if sneaking != server.state.paused {
                    server.state.paused = sneaking;
                    client.play_sound(
                        ident!("block.note_block.pling"),
                        SoundCategory::Block,
                        client.position(),
                        0.5f32,
                        if sneaking { 0.5f32 } else { 1f32 },
                    );
                }
            }

            // Display Playing in green or Paused in red
            client.set_action_bar(if server.state.paused {
                "Paused".color(Color::RED)
            } else {
                "Playing".color(Color::GREEN)
            });

            true
        });

        if !server.state.paused && server.shared.current_tick() % 2 == 0 {
            server
                .state
                .board_buf
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, cell)| {
                    let cx = (i % SIZE_X) as i32;
                    let cz = (i / SIZE_Z) as i32;

                    let mut live_count = 0;
                    for z in cz - 1..=cz + 1 {
                        for x in cx - 1..=cx + 1 {
                            if !(x == cx && z == cz) {
                                let i = x.rem_euclid(SIZE_X as i32) as usize
                                    + z.rem_euclid(SIZE_Z as i32) as usize * SIZE_X;
                                if server.state.board[i] {
                                    live_count += 1;
                                }
                            }
                        }
                    }

                    if server.state.board[cx as usize + cz as usize * SIZE_X] {
                        *cell = (2..=3).contains(&live_count);
                    } else {
                        *cell = live_count == 3;
                    }
                });

            mem::swap(&mut server.state.board, &mut server.state.board_buf);
        }

        let min_y = server.shared.dimensions().next().unwrap().1.min_y;

        for chunk_x in 0..Integer::div_ceil(&SIZE_X, &16) {
            for chunk_z in 0..Integer::div_ceil(&SIZE_Z, &16) {
                let chunk = world
                    .chunks
                    .get_mut((chunk_x as i32, chunk_z as i32))
                    .unwrap();
                for x in 0..16 {
                    for z in 0..16 {
                        let cell_x = chunk_x * 16 + x;
                        let cell_z = chunk_z * 16 + z;

                        if cell_x < SIZE_X && cell_z < SIZE_Z {
                            let b = if server.state.board[cell_x + cell_z * SIZE_X] {
                                BlockState::GRASS_BLOCK
                            } else {
                                BlockState::DIRT
                            };
                            chunk.set_block_state(x, (BOARD_Y - min_y) as usize, z, b);
                        }
                    }
                }
            }
        }
    }
}
