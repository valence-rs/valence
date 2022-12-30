use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use valence::prelude::*;

pub fn main() -> ShutdownResult {
    tracing_subscriber::fmt().init();

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
    // World and position to respawn at
    respawn_location: (WorldId, Vec3<f64>),
    // Anticheat measure
    can_respawn: bool,
}

#[derive(Default)]
struct ServerState {
    player_list: Option<PlayerListId>,
    first_world: WorldId,
    second_world: WorldId,
    third_world: WorldId,
}

const MAX_PLAYERS: usize = 10;

const FLOOR_Y: i32 = 64;
const PLATFORM_X: i32 = 20;
const PLATFORM_Z: i32 = 20;
const LEFT_DEATH_LINE: i32 = 16;
const RIGHT_DEATH_LINE: i32 = 4;

const FIRST_WORLD_SPAWN_BLOCK: BlockPos = BlockPos::new(10, FLOOR_Y, 10);
const SECOND_WORLD_SPAWN_BLOCK: BlockPos = BlockPos::new(5, FLOOR_Y, 5);
const THIRD_WORLD_SPAWN_BLOCK: BlockPos = BlockPos::new(5, FLOOR_Y, 5);

#[derive(Clone, Copy, PartialEq, Eq)]
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
    type WorldState = ();
    type ChunkState = ();
    type PlayerListState = ();
    type InventoryState = ();

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
            favicon_png: Some(include_bytes!("../../../assets/logo-64x64.png").as_slice().into()),
            player_sample: Default::default(),
        }
    }

    fn init(&self, server: &mut Server<Self>) {
        // We created server with meaningless default state.
        // Let's create three worlds and create new ServerState.
        server.state = ServerState {
            player_list: Some(server.player_lists.insert(()).0),
            first_world: create_world(server, FIRST_WORLD_SPAWN_BLOCK, WhichWorld::First),
            second_world: create_world(server, SECOND_WORLD_SPAWN_BLOCK, WhichWorld::Second),
            third_world: create_world(server, THIRD_WORLD_SPAWN_BLOCK, WhichWorld::Third),
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
                    Some((id, entity)) => {
                        entity.set_world(server.state.first_world);
                        client.entity_id = id
                    }
                    None => {
                        client.disconnect("Conflicting UUID");
                        return false;
                    }
                }

                client.respawn_location = (
                    server.state.first_world,
                    block_pos_to_vec(FIRST_WORLD_SPAWN_BLOCK),
                );

                // `set_spawn_position` is used for compass _only_
                client.set_spawn_position(FIRST_WORLD_SPAWN_BLOCK, 0.0);

                client.set_flat(true);
                client.respawn(server.state.first_world);
                client.teleport(client.respawn_location.1, 0.0, 0.0);

                client.set_player_list(server.state.player_list.clone());

                server
                    .player_lists
                    .get_mut(server.state.player_list.as_ref().unwrap())
                    .insert(
                        client.uuid(),
                        client.username(),
                        client.textures().cloned(),
                        client.game_mode(),
                        0,
                        None,
                        true,
                    );

                client.set_respawn_screen(true);

                client.send_message("Welcome to the death example!".italic());
                client.send_message("Step over the left line to die. :)");
                client.send_message("Step over the right line to die and respawn in second world.");
                client.send_message("Jumping down kills you and spawns you in another dimension.");
                client.send_message("Sneaking triggers game credits after which you respawn.");
            }

            // TODO after inventory support is added, show interaction with compass.

            // Handling respawn locations
            if !client.can_respawn {
                if client.position().y < 0.0 {
                    client.can_respawn = true;
                    client.kill(None, "You fell");
                    // You could have also killed the player with `Client::set_health_and_food`,
                    // however you cannot send a message to the death screen
                    // that way
                    if client.world() == server.state.third_world {
                        // Falling in third world gets you back to the first world
                        client.respawn_location = (
                            server.state.first_world,
                            block_pos_to_vec(FIRST_WORLD_SPAWN_BLOCK),
                        );
                        client.set_spawn_position(FIRST_WORLD_SPAWN_BLOCK, 0.0);
                    } else {
                        // falling in first and second world will cause player to spawn in third
                        // world
                        client.respawn_location = (
                            server.state.third_world,
                            block_pos_to_vec(THIRD_WORLD_SPAWN_BLOCK),
                        );
                        // This is for compass to point at
                        client.set_spawn_position(THIRD_WORLD_SPAWN_BLOCK, 0.0);
                    }
                }

                // Death lanes in the first world
                if client.world() == server.state.first_world {
                    let death_msg = "You shouldn't cross suspicious lines";

                    if client.position().x >= LEFT_DEATH_LINE as f64 {
                        // Client went to the left, he dies
                        client.can_respawn = true;
                        client.kill(None, death_msg);
                    }

                    if client.position().x <= RIGHT_DEATH_LINE as f64 {
                        // Client went to the right, he dies and spawns in world2
                        client.can_respawn = true;
                        client.kill(None, death_msg);
                        client.respawn_location = (
                            server.state.second_world,
                            block_pos_to_vec(SECOND_WORLD_SPAWN_BLOCK),
                        );
                        client.set_spawn_position(SECOND_WORLD_SPAWN_BLOCK, 0.0);
                    }
                }
            }

            let player = server.entities.get_mut(client.entity_id).unwrap();

            while let Some(event) = client.next_event() {
                event.handle_default(client, player);
                match event {
                    ClientEvent::PerformRespawn => {
                        if !client.can_respawn {
                            client.disconnect("Unexpected PerformRespawn");
                            return false;
                        }
                        // Let's respawn our player. `spawn` will load the world, but we are
                        // responsible for teleporting the player.

                        // You can store respawn however you want, for example in `Client`'s state.
                        let spawn = client.respawn_location;
                        client.respawn(spawn.0);
                        player.set_world(spawn.0);
                        client.teleport(spawn.1, 0.0, 0.0);
                        client.can_respawn = false;
                    }
                    ClientEvent::StartSneaking => {
                        // Roll the credits, respawn after
                        client.can_respawn = true;
                        client.win_game(true);
                    }
                    _ => {}
                }
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                server.entities[client.entity_id].set_deleted(true);

                if let Some(list) = client.player_list() {
                    server.player_lists.get_mut(list).remove(client.uuid());
                }

                return false;
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
        WhichWorld::Third => server.shared.dimensions().nth(1).unwrap(),
    };

    let (world_id, world) = server.worlds.insert(dimension.0, ());

    // Create chunks
    for chunk_z in -3..3 {
        for chunk_x in -3..3 {
            world
                .chunks
                .insert([chunk_x, chunk_z], UnloadedChunk::default(), ());
        }
    }

    // Create platform
    let platform_block = match world_type {
        WhichWorld::First => BlockState::END_STONE,
        WhichWorld::Second => BlockState::AMETHYST_BLOCK,
        WhichWorld::Third => BlockState::BLACKSTONE,
    };

    for z in 0..PLATFORM_Z {
        for x in 0..PLATFORM_X {
            world
                .chunks
                .set_block_state([x, FLOOR_Y, z], platform_block);
        }
    }

    // Set death lines
    if world_type == WhichWorld::First {
        for z in 0..PLATFORM_Z {
            world
                .chunks
                .set_block_state([LEFT_DEATH_LINE, FLOOR_Y, z], BlockState::GOLD_BLOCK);
            world
                .chunks
                .set_block_state([RIGHT_DEATH_LINE, FLOOR_Y, z], BlockState::DIAMOND_BLOCK);
        }
    }

    // Set spawn block
    world
        .chunks
        .set_block_state(spawn_pos, BlockState::REDSTONE_BLOCK);

    world_id
}
