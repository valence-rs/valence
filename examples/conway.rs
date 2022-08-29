use std::mem;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use num::Integer;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use valence::biome::Biome;
use valence::block::BlockState;
use valence::client::{Client, ClientEvent, Hand};
use valence::config::{Config, ServerListPing};
use valence::dimension::{Dimension, DimensionId};
use valence::entity::types::Pose;
use valence::entity::{Entity, EntityEvent, EntityId, EntityKind, TrackedData};
use valence::player_list::PlayerListId;
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
        ServerState {
            player_list: None,
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
    board: Box<[bool]>,
    board_buf: Box<[bool]>,
}

const MAX_PLAYERS: usize = 10;

const SIZE_X: usize = 100;
const SIZE_Z: usize = 100;
const BOARD_Y: i32 = 50;

#[async_trait]
impl Config for Game {
    type ServerState = ServerState;
    type ClientState = EntityId;
    type EntityState = ();
    type WorldState = ();
    type ChunkState = ();
    type PlayerListState = ();

    fn max_connections(&self) -> usize {
        // We want status pings to be successful even if the server is full.
        MAX_PLAYERS + 64
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
            favicon_png: Some(include_bytes!("../assets/favicon.png")),
        }
    }

    fn init(&self, server: &mut Server<Self>) {
        let world = server.worlds.insert(DimensionId::default(), ()).1;
        server.state.player_list = Some(server.player_lists.insert(()).0);

        for chunk_z in -2..Integer::div_ceil(&(SIZE_X as i32), &16) + 2 {
            for chunk_x in -2..Integer::div_ceil(&(SIZE_Z as i32), &16) + 2 {
                world.chunks.insert((chunk_x as i32, chunk_z as i32), ());
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
                    Some((id, _)) => client.state = id,
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
                client.send_message("Hold the left mouse button to bring blocks to life.".italic());
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                server.entities.remove(client.state);
                if let Some(id) = &server.state.player_list {
                    server.player_lists.get_mut(id).remove(client.uuid());
                }
                return false;
            }

            let player = server.entities.get_mut(client.state).unwrap();

            if client.position().y <= 0.0 {
                client.teleport(spawn_pos, client.yaw(), client.pitch());
            }

            while let Some(event) = client_event_boilerplate(client, player) {
                if let ClientEvent::Digging { position, .. } = event {
                    if (0..SIZE_X as i32).contains(&position.x)
                        && (0..SIZE_Z as i32).contains(&position.z)
                        && position.y == BOARD_Y
                    {
                        server.state.board[position.x as usize + position.z as usize * SIZE_X] =
                            true;
                    }
                }
            }

            true
        });

        if server.shared.current_tick() % 4 != 0 {
            return;
        }

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

fn client_event_boilerplate(
    client: &mut Client<Game>,
    entity: &mut Entity<Game>,
) -> Option<ClientEvent> {
    let event = client.pop_event()?;

    match &event {
        ClientEvent::ChatMessage { .. } => {}
        ClientEvent::SettingsChanged {
            view_distance,
            main_hand,
            displayed_skin_parts,
            ..
        } => {
            client.set_view_distance(*view_distance);

            let player = client.player_mut();

            player.set_cape(displayed_skin_parts.cape());
            player.set_jacket(displayed_skin_parts.jacket());
            player.set_left_sleeve(displayed_skin_parts.left_sleeve());
            player.set_right_sleeve(displayed_skin_parts.right_sleeve());
            player.set_left_pants_leg(displayed_skin_parts.left_pants_leg());
            player.set_right_pants_leg(displayed_skin_parts.right_pants_leg());
            player.set_hat(displayed_skin_parts.hat());
            player.set_main_arm(*main_hand as u8);

            if let TrackedData::Player(player) = entity.data_mut() {
                player.set_cape(displayed_skin_parts.cape());
                player.set_jacket(displayed_skin_parts.jacket());
                player.set_left_sleeve(displayed_skin_parts.left_sleeve());
                player.set_right_sleeve(displayed_skin_parts.right_sleeve());
                player.set_left_pants_leg(displayed_skin_parts.left_pants_leg());
                player.set_right_pants_leg(displayed_skin_parts.right_pants_leg());
                player.set_hat(displayed_skin_parts.hat());
                player.set_main_arm(*main_hand as u8);
            }
        }
        ClientEvent::MovePosition {
            position,
            on_ground,
        } => {
            entity.set_position(*position);
            entity.set_on_ground(*on_ground);
        }
        ClientEvent::MovePositionAndRotation {
            position,
            yaw,
            pitch,
            on_ground,
        } => {
            entity.set_position(*position);
            entity.set_yaw(*yaw);
            entity.set_head_yaw(*yaw);
            entity.set_pitch(*pitch);
            entity.set_on_ground(*on_ground);
        }
        ClientEvent::MoveRotation {
            yaw,
            pitch,
            on_ground,
        } => {
            entity.set_yaw(*yaw);
            entity.set_head_yaw(*yaw);
            entity.set_pitch(*pitch);
            entity.set_on_ground(*on_ground);
        }
        ClientEvent::MoveOnGround { on_ground } => {
            entity.set_on_ground(*on_ground);
        }
        ClientEvent::MoveVehicle { .. } => {}
        ClientEvent::StartSneaking => {
            if let TrackedData::Player(player) = entity.data_mut() {
                if player.get_pose() == Pose::Standing {
                    player.set_pose(Pose::Sneaking);
                }
            }
        }
        ClientEvent::StopSneaking => {
            if let TrackedData::Player(player) = entity.data_mut() {
                if player.get_pose() == Pose::Sneaking {
                    player.set_pose(Pose::Standing);
                }
            }
        }
        ClientEvent::StartSprinting => {
            if let TrackedData::Player(player) = entity.data_mut() {
                player.set_sprinting(true);
            }
        }
        ClientEvent::StopSprinting => {
            if let TrackedData::Player(player) = entity.data_mut() {
                player.set_sprinting(false);
            }
        }
        ClientEvent::StartJumpWithHorse { .. } => {}
        ClientEvent::StopJumpWithHorse => {}
        ClientEvent::LeaveBed => {}
        ClientEvent::OpenHorseInventory => {}
        ClientEvent::StartFlyingWithElytra => {}
        ClientEvent::ArmSwing(hand) => {
            entity.push_event(match hand {
                Hand::Main => EntityEvent::SwingMainHand,
                Hand::Off => EntityEvent::SwingOffHand,
            });
        }
        ClientEvent::InteractWithEntity { .. } => {}
        ClientEvent::SteerBoat { .. } => {}
        ClientEvent::Digging { .. } => {}
    }

    entity.set_world(client.world());

    Some(event)
}
