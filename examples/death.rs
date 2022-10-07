use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use num::Integer;
use valence::biome::Biome;
use valence::block::{BlockPos, BlockState};
use valence::chunk::{Chunk, UnloadedChunk};
use valence::client::{handle_event_default, ClientEvent};
use valence::config::{Config, ServerListPing};
use valence::dimension::Dimension;
use valence::entity::{EntityId, EntityKind};
use valence::player_list::PlayerListId;
use valence::protocol::packets::s2c::play::{GameEvent, GameStateChangeReason};
use valence::server::{Server, SharedServer, ShutdownResult};
use valence::text::{Color, TextFormat};
use valence::world::WorldId;
use valence::{async_trait, ident};
use vek::Vec3;

pub fn main() -> ShutdownResult {
    env_logger::Builder::new()
        .filter_module("valence", LevelFilter::Info) // todo reset to Trace
        .parse_default_env()
        .init();

    valence::start_server(
        Game {
            player_count: AtomicUsize::new(0),
        },
        ServerState::default(),
    )
}

struct Game {
    player_count: AtomicUsize,
}

#[derive(Default)]
struct ClientState {
    entity_id: EntityId,
    enable_death_screen_packet_sent: bool,
    // World and position to respawn at
    respawn_location: (WorldId, Vec3<f64>),
    // Anticheat measure
    can_respawn: bool,
}

struct WorldState {
    player_list: PlayerListId,
}

#[derive(Default)]
struct ServerState {
    first_world: WorldId,
    first_world_spawn_block: BlockPos,
    second_world: WorldId,
    second_world_spawn_block: BlockPos,
    third_world: WorldId,
    third_world_spawn_block: BlockPos,
}

const MAX_PLAYERS: usize = 10;

const FLOOR_Y: i32 = 64;
const PLATFORM_X: i32 = 20;
const PLATFORM_Z: i32 = 20;
const LEFT_DEATH_LINE: i32 = 16;
const RIGHT_DEATH_LINE: i32 = 4;

enum WhichWorld {
    First,
    Second,
    Third,
}

// Returns position of player standing on `pos` block
fn block_pos_to_vec(pos: BlockPos) -> Vec3<f64> {
    Vec3::new(
        (pos.x as f64) + 0.5,
        (pos.y as f64) + 1.0,
        (pos.z as f64) + 0.5,
    )
}

#[async_trait]
impl Config for Game {
    type ServerState = ServerState;
    type ClientState = ClientState;
    type EntityState = ();
    type WorldState = WorldState;
    type ChunkState = ();
    type PlayerListState = ();

    fn online_mode(&self) -> bool {
        false
    }

    fn max_connections(&self) -> usize {
        // We want status pings to be successful even if the server is full.
        MAX_PLAYERS + 64
    }

    fn dimensions(&self) -> Vec<Dimension> {
        vec![
            Dimension {
                fixed_time: Some(6000),
                ..Dimension::default()
            },
            Dimension {
                fixed_time: Some(19000),
                ..Dimension::default()
            },
        ]
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
        // We created server with meaningless default state.
        // Let's create three worlds and create new ServerState.

        let first_world_spawn_block = [10_i32, FLOOR_Y, 10].into();
        let first_world = create_world(server, first_world_spawn_block, WhichWorld::First);

        let second_world_spawn_block = [5_i32, FLOOR_Y, 5].into();
        let second_world = create_world(server, second_world_spawn_block, WhichWorld::Second);

        let third_world_spawn_block = [5_i32, FLOOR_Y, 5].into();
        let third_world = create_world(server, third_world_spawn_block, WhichWorld::Third);

        server.state = ServerState {
            first_world,
            first_world_spawn_block,
            second_world,
            second_world_spawn_block,
            third_world,
            third_world_spawn_block,
        };
    }

    fn update(&self, server: &mut Server<Self>) {
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

                let first_world_id = server.state.first_world;
                let first_world = server.worlds.get(first_world_id).unwrap();

                client.state.respawn_location = (
                    server.state.first_world,
                    block_pos_to_vec(server.state.first_world_spawn_block),
                );

                // `set_spawn_position` is used for compass _only_
                client.set_spawn_position(server.state.first_world_spawn_block, 0.0);

                client.set_flat(true);
                client.spawn(first_world_id);
                client.teleport(client.state.respawn_location.1, 0.0, 0.0);

                client.set_player_list(first_world.state.player_list.clone());

                server
                    .player_lists
                    .get_mut(&first_world.state.player_list)
                    .insert(
                        client.uuid(),
                        client.username(),
                        client.textures().cloned(),
                        client.game_mode(),
                        0,
                        None,
                    );

                client.send_message("Welcome to the death example!".italic());
                client.send_message("Step over the left line to die. :)");
                client.send_message("Step over the right line to die and respawn in second world.");
                client.send_message("Jumping down kills you and spawns you in another dimension.");
                client.send_message("Sneaking triggers game credits after which you respawn.");
            }

            if !client.state.enable_death_screen_packet_sent && !client.created_this_tick() {
                // This packet enables death screen
                // TODO create helper function, this must be deferred
                client.send_packet(GameEvent {
                    reason: GameStateChangeReason::EnableRespawnScreen,
                    value: 0.0,
                });
                client.state.enable_death_screen_packet_sent = true;
            }

            // TODO after inventory support is added, show interaction with compass.

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                server.entities.remove(client.state.entity_id);

                if let Some(list) = client.player_list() {
                    server.player_lists.get_mut(list).remove(client.uuid());
                }

                return false;
            }

            // Handling respawn locations
            if !client.state.can_respawn {
                if client.position().y <= -10.0 {
                    client.state.can_respawn = true;
                    client.kill(None, "You fell");
                    // You could have also killed the player with `Client::set_health_and_food`,
                    // however you cannot send a message to the death screen
                    // that way
                    if client.world() == server.state.third_world {
                        // Falling in third world gets you back to the first world
                        client.state.respawn_location = (
                            server.state.first_world,
                            block_pos_to_vec(server.state.first_world_spawn_block),
                        );
                        client.set_spawn_position(server.state.first_world_spawn_block, 0.0);
                    } else {
                        // falling in first and second world will cause player to spawn in third
                        // world
                        client.state.respawn_location = (
                            server.state.third_world,
                            block_pos_to_vec(server.state.third_world_spawn_block),
                        );
                        // This is for compass to point at
                        client.set_spawn_position(server.state.third_world_spawn_block, 0.0);
                    }
                }

                // Death lanes in the first world
                if client.world() == server.state.first_world {
                    if client.position().x >= LEFT_DEATH_LINE as f64 {
                        // Client went to the left, he dies
                        client.state.can_respawn = true;
                        client.kill(None, "You shouldn't cross suspicious lines");
                    }

                    if client.position().x <= RIGHT_DEATH_LINE as f64 {
                        // Client went to the right, he dies and spawns in world2
                        client.state.can_respawn = true;
                        client.kill(None, "You shouldn't cross suspicious lines");
                        client.state.respawn_location = (
                            server.state.second_world,
                            block_pos_to_vec(server.state.second_world_spawn_block),
                        );
                        client.set_spawn_position(server.state.second_world_spawn_block, 0.0);
                    }
                }
            }

            let player = server.entities.get_mut(client.state.entity_id).unwrap();

            while let Some(event) = handle_event_default(client, player) {
                match event {
                    ClientEvent::RespawnRequest => {
                        if !client.state.can_respawn {
                            client.disconnect("Unexpected RespawnRequest");
                        }
                        // Let's respawn our player. `spawn` will load the world, but we are
                        // responsible for teleporting the player.

                        // You can store respawn however you want, for example in `Client`'s state.
                        let spawn = client.state.respawn_location;
                        client.spawn(spawn.0);
                        client.teleport(spawn.1, 0.0, 0.0);
                        client.state.can_respawn = false;
                    }
                    ClientEvent::StartSneaking => {
                        // Roll the credits, respawn after
                        client.state.can_respawn = true;
                        client.send_packet(GameEvent {
                            reason: GameStateChangeReason::WinGame,
                            value: 1.0,
                        });
                    }
                    _ => {}
                }
            }

            true
        });
    }
}

// Boilerplate for creating world
fn create_world(server: &mut Server<Game>, spawn_pos: BlockPos, world_type: WhichWorld) -> WorldId {
    let dimension = match world_type {
        WhichWorld::First => server.shared.dimensions().next().unwrap(),
        WhichWorld::Second => server.shared.dimensions().next().unwrap(),
        WhichWorld::Third => server.shared.dimensions().skip(1).next().unwrap(),
    };

    let player_list = server.player_lists.insert(()).0;
    let (world_id, world1) = server
        .worlds
        .insert(dimension.0, WorldState { player_list });

    let first_min_y = server.shared.dimension(world1.meta.dimension()).min_y;

    // Create chunks
    for chunk_z in -2..2 {
        for chunk_x in -2..2 {
            world1.chunks.insert(
                [chunk_x as i32, chunk_z as i32],
                UnloadedChunk::default(),
                (),
            );
        }
    }

    // Create platform
    for chunk_x in 0..Integer::div_ceil(&PLATFORM_X, &16) {
        for chunk_z in 0..Integer::div_ceil(&PLATFORM_Z, &16) {
            let chunk = world1.chunks.get_mut((chunk_x, chunk_z)).unwrap();
            for x in 0..16_usize {
                for z in 0..16_usize {
                    let cell_x = chunk_x * 16 + x as i32;
                    let cell_z = chunk_z * 16 + z as i32;

                    let b = if cell_x == spawn_pos.x && cell_z == spawn_pos.z {
                        BlockState::REDSTONE_BLOCK
                    } else {
                        match world_type {
                            WhichWorld::First => match cell_x {
                                LEFT_DEATH_LINE => BlockState::GOLD_BLOCK,
                                RIGHT_DEATH_LINE => BlockState::DIAMOND_BLOCK,
                                _ => BlockState::END_STONE,
                            },
                            _ => BlockState::BLACKSTONE,
                        }
                    };

                    if cell_x < PLATFORM_X && cell_z < PLATFORM_Z {
                        chunk.set_block_state(x, (FLOOR_Y - first_min_y) as usize, z, b);
                    }
                }
            }
        }
    }

    world_id
}
