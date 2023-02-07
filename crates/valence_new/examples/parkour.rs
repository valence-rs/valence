use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::seq::SliceRandom;
use rand::Rng;
use valence_new::client::despawn_disconnected_clients;
use valence_new::client::event::default_event_handler;
use valence_new::prelude::*;

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

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_system_set(PlayerList::default_system_set())
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_system(reset_players)
        .add_system(manage_blocks)
        .run();
}

#[derive(Component)]
struct Player {
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
        let instance = server.new_instance(DimensionId::default());
        commands.entity(ent).insert(instance);

        client.set_position([
            START_POS.x as f64 + 0.5,
            START_POS.y as f64 + 1.0,
            START_POS.z as f64 + 0.5,
        ]);
        client.set_respawn_screen(true);
        client.set_instance(ent);
        client.set_game_mode(GameMode::Adventure);
        client.send_message("Welcome to Valence!".italic());

        let player = Player {
            blocks: VecDeque::new(),
            score: 0,
            combo: 0,
            target_y: 0,
            last_block_timestamp: 0,
        };
        commands.entity(ent).insert(player);
    }
}

fn reset_players(mut clients: Query<(&mut Client, &mut Player, &mut Instance), With<Player>>) {
    for (mut client, mut player, mut instance) in clients.iter_mut() {
        if player.is_added() {
            reset(&mut client, &mut player, &mut instance);
            continue;
        }

        if (client.position().y as i32) < START_POS.y - 32 {
            reset(&mut client, &mut player, &mut instance);
            continue;
        }
    }
}

fn manage_blocks(mut clients: Query<(&mut Client, &mut Player, &mut Instance)>) {
    for (client, mut player, mut instance) in clients.iter_mut() {
        let pos_under_player = BlockPos::new(
            (client.position().x - 0.5).round() as i32,
            client.position().y as i32 - 1,
            (client.position().z - 0.5).round() as i32,
        );

        if let Some(index) = player
            .blocks
            .iter()
            .position(|block| *block == pos_under_player)
        {
            if index > 0 {
                let power_result = 2.0f32.powf((player.combo as f32) / 45.0);
                let max_time_taken = (1000.0f32 * (index as f32) / power_result) as u128;

                let current_time_millis = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                if current_time_millis - player.last_block_timestamp < max_time_taken {
                    player.combo += index as u32
                } else {
                    player.combo = 0
                }

                // let pitch = 0.9 + ((client.combo as f32) - 1.0) * 0.05;

                for _ in 0..index {
                    generate_next_block(&mut player, &mut instance, true)
                }

                // TODO: add sounds again.
                // client.play_sound(
                //     Ident::new("minecraft:block.note_block.bass").unwrap(),
                //     SoundCategory::Master,
                //     client.position(),
                //     1f32,
                //     pitch,
                // );

                // client.set_title(
                //     "",
                //     client.score.to_string().color(Color::LIGHT_PURPLE).
                // bold(),     SetTitleAnimationTimes {
                //         fade_in: 0,
                //         stay: 7,
                //         fade_out: 4,
                //     },
                // );
            }
        }
    }
}

fn reset(client: &mut Client, player: &mut Player, instance: &mut Instance) {
    // Load chunks around spawn to avoid double void reset
    for chunk_z in -1..3 {
        for chunk_x in -2..2 {
            instance.insert_chunk([chunk_x, chunk_z], Chunk::default());
        }
    }

    player.score = 0;
    player.combo = 0;

    for block in &player.blocks {
        instance.set_block_state(*block, BlockState::AIR);
    }
    player.blocks.clear();
    player.blocks.push_back(START_POS);
    instance.set_block_state(START_POS, BlockState::STONE);

    for _ in 0..10 {
        generate_next_block(player, instance, false);
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

fn generate_next_block(player: &mut Player, instance: &mut Instance, in_game: bool) {
    if in_game {
        let removed_block = player.blocks.pop_front().unwrap();
        instance.set_block_state(removed_block, BlockState::AIR);

        player.score += 1
    }

    let last_pos = *player.blocks.back().unwrap();
    let block_pos = generate_random_block(last_pos, player.target_y);

    if last_pos.y == START_POS.y {
        player.target_y = 0
    } else if last_pos.y < START_POS.y - 30 || last_pos.y > START_POS.y + 30 {
        player.target_y = START_POS.y;
    }

    let mut rng = rand::thread_rng();

    instance.set_block_state(block_pos, *BLOCK_TYPES.choose(&mut rng).unwrap());
    player.blocks.push_back(block_pos);

    // Combo System
    player.last_block_timestamp = SystemTime::now()
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
