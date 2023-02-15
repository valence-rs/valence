use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::seq::SliceRandom;
use rand::Rng;
use valence::client::despawn_disconnected_clients;
use valence::client::event::default_event_handler;
use valence::prelude::*;
use valence_protocol::packets::s2c::play::SetTitleAnimationTimes;
use valence_protocol::types::SoundCategory;
use valence_protocol::Sound;

const START_POS: BlockPos = BlockPos::new(0, 100, 0);
const VIEW_DIST: u8 = 10;

const BLOCK_TYPES: [BlockState; 7] = [
    BlockState::GRASS_BLOCK,
    BlockState::OAK_LOG,
    BlockState::BIRCH_LOG,
    BlockState::OAK_LEAVES,
    BlockState::BIRCH_LEAVES,
    BlockState::DIRT,
    BlockState::MOSS_BLOCK,
];

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_system_set(PlayerList::default_system_set())
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_system(reset_clients.after(init_clients))
        .add_system(manage_chunks.after(reset_clients).before(manage_blocks))
        .add_system(manage_blocks)
        .run();
}

#[derive(Component)]
struct GameState {
    blocks: VecDeque<BlockPos>,
    score: u32,
    combo: u32,
    target_y: i32,
    last_block_timestamp: u128,
}

fn init_clients(
    mut commands: Commands,
    server: Res<Server>,
    mut clients: Query<(Entity, &mut Client), Added<Client>>,
) {
    for (ent, mut client) in clients.iter_mut() {
        let mut instance = server.new_instance(DimensionId::default());

        for pos in client.view().with_dist(VIEW_DIST).iter() {
            assert!(instance.insert_chunk(pos, Chunk::default()).is_none());
        }

        client.set_position([
            START_POS.x as f64 + 0.5,
            START_POS.y as f64 + 1.0,
            START_POS.z as f64 + 0.5,
        ]);
        client.set_flat(true);
        client.set_instance(ent);
        client.set_game_mode(GameMode::Adventure);
        client.send_message("Welcome to epic infinite parkour game!".italic());

        let mut state = GameState {
            blocks: VecDeque::new(),
            score: 0,
            combo: 0,
            target_y: 0,
            last_block_timestamp: 0,
        };

        reset(&mut client, &mut state, &mut instance);

        commands.entity(ent).insert(state);
        commands.entity(ent).insert(instance);
    }
}

fn reset_clients(
    mut clients: Query<(&mut Client, &mut GameState, &mut Instance), With<GameState>>,
) {
    for (mut client, mut state, mut instance) in clients.iter_mut() {
        if (client.position().y as i32) < START_POS.y - 32 {
            client.send_message(
                "Your score was ".italic()
                    + state
                        .score
                        .to_string()
                        .color(Color::GOLD)
                        .bold()
                        .not_italic(),
            );

            reset(&mut client, &mut state, &mut instance);
        }
    }
}

fn manage_blocks(mut clients: Query<(&mut Client, &mut GameState, &mut Instance)>) {
    for (mut client, mut state, mut instance) in clients.iter_mut() {
        let pos_under_player = BlockPos::new(
            (client.position().x - 0.5).round() as i32,
            client.position().y as i32 - 1,
            (client.position().z - 0.5).round() as i32,
        );

        if let Some(index) = state
            .blocks
            .iter()
            .position(|block| *block == pos_under_player)
        {
            if index > 0 {
                let power_result = 2.0f32.powf((state.combo as f32) / 45.0);
                let max_time_taken = (1000.0f32 * (index as f32) / power_result) as u128;

                let current_time_millis = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis();

                if current_time_millis - state.last_block_timestamp < max_time_taken {
                    state.combo += index as u32
                } else {
                    state.combo = 0
                }

                for _ in 0..index {
                    generate_next_block(&mut state, &mut instance, true)
                }

                let pitch = 0.9 + ((state.combo as f32) - 1.0) * 0.05;
                let pos = client.position();
                client.play_sound(
                    Sound::BlockNoteBlockBass,
                    SoundCategory::Master,
                    pos,
                    1.0,
                    pitch,
                );

                client.set_title(
                    "",
                    state.score.to_string().color(Color::LIGHT_PURPLE).bold(),
                    SetTitleAnimationTimes {
                        fade_in: 0,
                        stay: 7,
                        fade_out: 4,
                    },
                );
            }
        }
    }
}

fn manage_chunks(mut clients: Query<(&mut Client, &mut Instance)>) {
    for (client, mut instance) in &mut clients {
        let old_view = client.old_view().with_dist(VIEW_DIST);
        let view = client.view().with_dist(VIEW_DIST);

        if old_view != view {
            for pos in old_view.diff(view) {
                instance.chunk_entry(pos).or_default();
            }

            for pos in view.diff(old_view) {
                instance.chunk_entry(pos).or_default();
            }
        }
    }
}

fn reset(client: &mut Client, state: &mut GameState, instance: &mut Instance) {
    // Load chunks around spawn to avoid double void reset
    for z in -1..3 {
        for x in -2..2 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    state.score = 0;
    state.combo = 0;

    for block in &state.blocks {
        instance.set_block_state(*block, BlockState::AIR);
    }
    state.blocks.clear();
    state.blocks.push_back(START_POS);
    instance.set_block_state(START_POS, BlockState::STONE);

    for _ in 0..10 {
        generate_next_block(state, instance, false);
    }

    client.set_position([
        START_POS.x as f64 + 0.5,
        START_POS.y as f64 + 1.0,
        START_POS.z as f64 + 0.5,
    ]);
    client.set_velocity([0f32, 0f32, 0f32]);
    client.set_yaw(0f32);
    client.set_pitch(0f32)
}

fn generate_next_block(state: &mut GameState, instance: &mut Instance, in_game: bool) {
    if in_game {
        let removed_block = state.blocks.pop_front().unwrap();
        instance.set_block_state(removed_block, BlockState::AIR);

        state.score += 1
    }

    let last_pos = *state.blocks.back().unwrap();
    let block_pos = generate_random_block(last_pos, state.target_y);

    if last_pos.y == START_POS.y {
        state.target_y = 0
    } else if last_pos.y < START_POS.y - 30 || last_pos.y > START_POS.y + 30 {
        state.target_y = START_POS.y;
    }

    let mut rng = rand::thread_rng();

    instance.set_block_state(block_pos, *BLOCK_TYPES.choose(&mut rng).unwrap());
    state.blocks.push_back(block_pos);

    // Combo System
    state.last_block_timestamp = SystemTime::now()
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
        1 => rng.gen_range(1..3),
        -1 => rng.gen_range(2..5),
        _ => rng.gen_range(1..4),
    };
    let x = rng.gen_range(-3..4);

    BlockPos::new(pos.x + x, pos.y + y, pos.z + z)
}
