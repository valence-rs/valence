use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use rand::seq::SliceRandom;
use rand::Rng;
use valence::prelude::*;

pub fn main() -> ShutdownResult {
    tracing_subscriber::fmt().init();

    valence::start_server(
        Game {
            player_count: AtomicUsize::new(0),
        },
        None,
    )
}

struct Game {
    player_count: AtomicUsize,
}

#[derive(Default)]
struct ChunkState {
    keep_loaded: bool,
}

#[derive(Default)]
struct ClientState {
    entity_id: EntityId,
    blocks: VecDeque<BlockPos>,
    score: u32,
    combo: u32,
    target_y: i32,
    last_block_timestamp: u128,
    world_id: WorldId,
}

const MAX_PLAYERS: usize = 10;
const START_POS: BlockPos = BlockPos::new(0, 100, 0);

const BLOCK_TYPES: [BlockState; 7] = [
    BlockState::GRASS_BLOCK,
    BlockState::OAK_LOG,
    BlockState::BIRCH_LOG,
    BlockState::OAK_LEAVES,
    BlockState::BIRCH_LEAVES,
    BlockState::DIRT,
    BlockState::MOSS_BLOCK,
];

#[async_trait]
impl Config for Game {
    type ServerState = Option<PlayerListId>;
    type ClientState = ClientState;
    type EntityState = ();
    type WorldState = ();
    type ChunkState = ChunkState;
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
        server.state = Some(server.player_lists.insert(()).0);
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

                let (world_id, world) = server.worlds.insert(DimensionId::default(), ());

                match server
                    .entities
                    .insert_with_uuid(EntityKind::Player, client.uuid(), ())
                {
                    Some((id, entity)) => {
                        entity.set_world(world_id);

                        // create client state
                        client.state = ClientState {
                            entity_id: id,
                            blocks: VecDeque::new(),
                            score: 0,
                            combo: 0,
                            last_block_timestamp: 0,
                            target_y: 0,
                            world_id,
                        };
                    }
                    None => {
                        client.disconnect("Conflicting UUID");
                        server.worlds.remove(world_id);
                        return false;
                    }
                }

                if let Some(id) = &server.state {
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

                client.respawn(world_id);
                client.set_flat(true);
                client.set_player_list(server.state.clone());

                client.send_message("Welcome to epic infinite parkour game!".italic());
                client.set_game_mode(GameMode::Adventure);
                reset(client, world);
            }

            let world_id = client.world_id;
            let world = &mut server.worlds[world_id];

            let p = client.position();
            for pos in ChunkPos::at(p.x, p.z).in_view(3) {
                if let Some(chunk) = world.chunks.get_mut(pos) {
                    chunk.keep_loaded = true;
                } else {
                    world.chunks.insert(
                        pos,
                        UnloadedChunk::default(),
                        ChunkState { keep_loaded: true },
                    );
                }
            }

            if (client.position().y as i32) < START_POS.y - 32 {
                client.send_message(
                    "Your score was ".italic()
                        + client
                            .score
                            .to_string()
                            .color(Color::GOLD)
                            .bold()
                            .not_italic(),
                );

                reset(client, world);
            }

            let pos_under_player = BlockPos::new(
                (client.position().x - 0.5).round() as i32,
                client.position().y as i32 - 1,
                (client.position().z - 0.5).round() as i32,
            );

            if let Some(index) = client
                .blocks
                .iter()
                .position(|block| *block == pos_under_player)
            {
                if index > 0 {
                    let power_result = 2.0f32.powf((client.combo as f32) / 45.0);
                    let max_time_taken = (1000.0f32 * (index as f32) / power_result) as u128;

                    let current_time_millis = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis();
                    if current_time_millis - client.last_block_timestamp < max_time_taken {
                        client.combo += index as u32
                    } else {
                        client.combo = 0
                    }

                    // let pitch = 0.9 + ((client.combo as f32) - 1.0) * 0.05;

                    for _ in 0..index {
                        generate_next_block(client, world, true)
                    }

                    // TODO: add sounds again.
                    // client.play_sound(
                    //     Ident::new("minecraft:block.note_block.bass").unwrap(),
                    //     SoundCategory::Master,
                    //     client.position(),
                    //     1f32,
                    //     pitch,
                    // );

                    client.set_title(
                        "",
                        client.score.to_string().color(Color::LIGHT_PURPLE).bold(),
                        SetTitleAnimationTimes {
                            fade_in: 0,
                            stay: 7,
                            fade_out: 4,
                        },
                    );
                }
            }

            let player = server.entities.get_mut(client.entity_id).unwrap();

            while let Some(event) = client.next_event() {
                event.handle_default(client, player);
            }

            // Remove chunks outside the view distance of players.
            for (_, chunk) in world.chunks.iter_mut() {
                chunk.set_deleted(!chunk.keep_loaded);
                chunk.keep_loaded = false;
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                if let Some(id) = &server.state {
                    server.player_lists[id].remove(client.uuid());
                }
                player.set_deleted(true);
                server.worlds.remove(world_id);
                return false;
            }

            true
        });
    }
}

fn reset(client: &mut Client<Game>, world: &mut World<Game>) {
    // Load chunks around spawn to avoid double void reset
    for chunk_z in -1..3 {
        for chunk_x in -2..2 {
            world.chunks.insert(
                [chunk_x, chunk_z],
                UnloadedChunk::default(),
                ChunkState { keep_loaded: true },
            );
        }
    }

    client.score = 0;
    client.combo = 0;

    for block in &client.blocks {
        world.chunks.set_block_state(*block, BlockState::AIR);
    }
    client.blocks.clear();
    client.blocks.push_back(START_POS);
    world.chunks.set_block_state(START_POS, BlockState::STONE);

    for _ in 0..10 {
        generate_next_block(client, world, false)
    }

    client.teleport(
        [
            START_POS.x as f64 + 0.5,
            START_POS.y as f64 + 1.0,
            START_POS.z as f64 + 0.5,
        ],
        0f32,
        0f32,
    );
}

fn generate_next_block(client: &mut Client<Game>, world: &mut World<Game>, in_game: bool) {
    if in_game {
        let removed_block = client.blocks.pop_front().unwrap();
        world.chunks.set_block_state(removed_block, BlockState::AIR);

        client.score += 1
    }

    let last_pos = *client.blocks.back().unwrap();
    let block_pos = generate_random_block(last_pos, client.target_y);

    if last_pos.y == START_POS.y {
        client.target_y = 0
    } else if last_pos.y < START_POS.y - 30 || last_pos.y > START_POS.y + 30 {
        client.target_y = START_POS.y;
    }

    let mut rng = rand::thread_rng();

    world
        .chunks
        .set_block_state(block_pos, *BLOCK_TYPES.choose(&mut rng).unwrap());
    client.blocks.push_back(block_pos);

    // Combo System
    client.last_block_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
}

fn generate_random_block(pos: BlockPos, target_y: i32) -> BlockPos {
    let mut rng = rand::thread_rng();

    // if above or below target_y, change y to gradually reach it
    let y = match target_y {
        0 => rng.gen_range(-1..2),
        y if y > pos.y => 1,
        _ => -1,
    };
    let z = match y {
        1 => rng.gen_range(1..4),
        -1 => rng.gen_range(2..6),
        _ => rng.gen_range(1..5),
    };
    let x = rng.gen_range(-3..4);

    BlockPos::new(pos.x + x, pos.y + y, pos.z + z)
}
