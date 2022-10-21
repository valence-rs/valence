use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use num::Integer;
use valence::client::{BlockFace, DiggingStatus, Hand};
use valence::prelude::*;

pub fn main() -> ShutdownResult {
    env_logger::Builder::new()
        .filter_module("valence", LevelFilter::Trace)
        .parse_default_env()
        .init();

    valence::start_server(
        Game {
            player_count: AtomicUsize::new(0),
        },
        ServerState { player_list: None },
    )
}

struct Game {
    player_count: AtomicUsize,
}

struct ServerState {
    player_list: Option<PlayerListId>,
}

#[derive(Default)]
struct ClientState {
    entity_id: EntityId,
}

const MAX_PLAYERS: usize = 10;

const SIZE_X: usize = 100;
const SIZE_Z: usize = 100;

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

    fn dimensions(&self) -> Vec<Dimension> {
        vec![Dimension {
            fixed_time: Some(6000),
            ..Dimension::default()
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

        // initialize chunks
        for chunk_z in -2..Integer::div_ceil(&(SIZE_Z as i32), &16) + 2 {
            for chunk_x in -2..Integer::div_ceil(&(SIZE_X as i32), &16) + 2 {
                world.chunks.insert(
                    [chunk_x as i32, chunk_z as i32],
                    UnloadedChunk::default(),
                    (),
                );
            }
        }

        // initialize blocks in the chunks
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
                            chunk.set_block_state(x, 63, z, BlockState::GRASS_BLOCK);
                        }
                    }
                }
            }
        }
    }

    fn update(&self, server: &mut Server<Self>) {
        let (world_id, world) = server.worlds.iter_mut().next().unwrap();

        let spawn_pos = [SIZE_X as f64 / 2.0, 1.0, SIZE_Z as f64 / 2.0];

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

                client.set_game_mode(GameMode::Creative);
                client.send_message("Welcome to Valence! Build something cool.".italic());
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

            if client.position().y <= -20.0 {
                client.teleport(spawn_pos, client.yaw(), client.pitch());
            }

            while let Some(event) = handle_event_default(client, player) {
                match event {
                    ClientEvent::Digging {
                        position, status, ..
                    } => {
                        match status {
                            DiggingStatus::Start => {
                                // Allows clients in creative mode to break blocks.
                                if client.game_mode() == GameMode::Creative {
                                    world.chunks.set_block_state(position, BlockState::AIR);
                                }
                            }
                            DiggingStatus::Finish => {
                                // Allows clients in survival mode to break blocks.
                                world.chunks.set_block_state(position, BlockState::AIR);
                            }
                            _ => {}
                        }
                    }
                    ClientEvent::InteractWithBlock {
                        hand,
                        location,
                        face,
                        ..
                    } => {
                        if hand == Hand::Main {
                            if let Some(stack) = client.held_item() {
                                if let Some(held_block_kind) = stack.item.to_block_kind() {
                                    let block_to_place = face_block(
                                        BlockState::from_kind(held_block_kind),
                                        face,
                                        client.yaw(),
                                    );

                                    if world
                                        .chunks
                                        .block_state(location)
                                        .map(|s| s.is_replaceable())
                                        .unwrap_or(false)
                                    {
                                        world.chunks.set_block_state(location, block_to_place);
                                    } else {
                                        let place_at = location.get_in_direction(face);
                                        world.chunks.set_block_state(place_at, block_to_place);
                                    }

                                    if client.game_mode() != GameMode::Creative {
                                        client.consume_one_held_item();
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            true
        });
    }
}

fn face_block(block: BlockState, face: BlockFace, yaw: f32) -> BlockState {
    let block = match block.to_wall_variant() {
        Some(wall_vaniant) => match face {
            BlockFace::Bottom | BlockFace::Top => block,
            _ => wall_vaniant,
        },
        None => block,
    };

    if let Some(props) = block.all_props() {
        let mut mut_block = block;

        for prop in props {
            mut_block = match prop {
                PropName::Facing => mut_block.set(PropName::Facing, face.to_block_facing()),
                PropName::Axis => mut_block.set(PropName::Axis, face.to_block_axis()),
                PropName::Rotation => mut_block.set(
                    PropName::Rotation,
                    PropValue::from_u16(
                        (((yaw as f64 * 16.0 / 360.0) + 0.5).floor() as i32 & 0xf) as u16,
                    )
                    .unwrap_or_else(|| panic!("Player yaw: {}, was out of bound", yaw)),
                ),
                _ => continue,
            }
        }

        mut_block
    } else {
        block
    }
}
