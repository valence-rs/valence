use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use num::Integer;
use valence::async_trait;
use valence::block::{BlockKind, BlockState, PropName, PropValue};
use valence::chunk::{Chunk, UnloadedChunk};
use valence::client::{
    handle_event_default, BlockFace, ClientEvent, DiggingStatus, GameMode, Hand,
};
use valence::config::{Config, ServerListPing};
use valence::dimension::{Dimension, DimensionId};
use valence::entity::{EntityId, EntityKind};
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
                            let place_at = location.get_in_direction(face);
                            if let Some(stack) = client.held_item() {
                                if let Some(block_kind) = stack.item.to_block_kind() {
                                    let state = match block_kind {
                                        // Torches
                                        BlockKind::Torch => face_wall_block(
                                            face,
                                            BlockState::TORCH,
                                            BlockState::WALL_TORCH,
                                        ),
                                        BlockKind::RedstoneTorch => face_wall_block(
                                            face,
                                            BlockState::REDSTONE_TORCH,
                                            BlockState::REDSTONE_WALL_TORCH,
                                        ),
                                        BlockKind::SoulTorch => face_wall_block(
                                            face,
                                            BlockState::SOUL_TORCH,
                                            BlockState::SOUL_WALL_TORCH,
                                        ),
                                        // Signs
                                        BlockKind::OakSign => face_wall_block(
                                            face,
                                            BlockState::OAK_SIGN,
                                            BlockState::OAK_WALL_SIGN,
                                        ),
                                        BlockKind::SpruceSign => face_wall_block(
                                            face,
                                            BlockState::SPRUCE_SIGN,
                                            BlockState::SPRUCE_WALL_SIGN,
                                        ),
                                        BlockKind::BirchSign => face_wall_block(
                                            face,
                                            BlockState::BIRCH_SIGN,
                                            BlockState::BIRCH_WALL_SIGN,
                                        ),
                                        BlockKind::AcaciaSign => face_wall_block(
                                            face,
                                            BlockState::ACACIA_SIGN,
                                            BlockState::ACACIA_WALL_SIGN,
                                        ),
                                        BlockKind::JungleSign => face_wall_block(
                                            face,
                                            BlockState::JUNGLE_SIGN,
                                            BlockState::JUNGLE_WALL_SIGN,
                                        ),
                                        BlockKind::DarkOakSign => face_wall_block(
                                            face,
                                            BlockState::DARK_OAK_SIGN,
                                            BlockState::DARK_OAK_WALL_SIGN,
                                        ),
                                        BlockKind::MangroveSign => face_wall_block(
                                            face,
                                            BlockState::MANGROVE_SIGN,
                                            BlockState::MANGROVE_WALL_SIGN,
                                        ),
                                        BlockKind::CrimsonSign => face_wall_block(
                                            face,
                                            BlockState::CRIMSON_SIGN,
                                            BlockState::CRIMSON_WALL_SIGN,
                                        ),
                                        BlockKind::WarpedSign => face_wall_block(
                                            face,
                                            BlockState::WARPED_SIGN,
                                            BlockState::WARPED_WALL_SIGN,
                                        ),
                                        // Skulls and heads
                                        BlockKind::SkeletonSkull => face_wall_block(
                                            face,
                                            BlockState::SKELETON_SKULL,
                                            BlockState::SKELETON_WALL_SKULL,
                                        ),
                                        BlockKind::WitherSkeletonSkull => face_wall_block(
                                            face,
                                            BlockState::WITHER_SKELETON_SKULL,
                                            BlockState::WITHER_SKELETON_WALL_SKULL,
                                        ),
                                        BlockKind::ZombieHead => face_wall_block(
                                            face,
                                            BlockState::ZOMBIE_HEAD,
                                            BlockState::ZOMBIE_WALL_HEAD,
                                        ),
                                        BlockKind::PlayerHead => face_wall_block(
                                            face,
                                            BlockState::PLAYER_HEAD,
                                            BlockState::PLAYER_WALL_HEAD,
                                        ),
                                        BlockKind::CreeperHead => face_wall_block(
                                            face,
                                            BlockState::CREEPER_HEAD,
                                            BlockState::CREEPER_WALL_HEAD,
                                        ),
                                        BlockKind::DragonHead => face_wall_block(
                                            face,
                                            BlockState::DRAGON_HEAD,
                                            BlockState::DRAGON_WALL_HEAD,
                                        ),
                                        // Banners
                                        BlockKind::WhiteBanner => face_wall_block(
                                            face,
                                            BlockState::WHITE_BANNER,
                                            BlockState::WHITE_WALL_BANNER,
                                        ),
                                        BlockKind::OrangeBanner => face_wall_block(
                                            face,
                                            BlockState::ORANGE_BANNER,
                                            BlockState::ORANGE_WALL_BANNER,
                                        ),
                                        BlockKind::MagentaBanner => face_wall_block(
                                            face,
                                            BlockState::MAGENTA_BANNER,
                                            BlockState::MAGENTA_WALL_BANNER,
                                        ),
                                        BlockKind::LightBlueBanner => face_wall_block(
                                            face,
                                            BlockState::LIGHT_BLUE_BANNER,
                                            BlockState::LIGHT_BLUE_WALL_BANNER,
                                        ),
                                        BlockKind::YellowBanner => face_wall_block(
                                            face,
                                            BlockState::YELLOW_BANNER,
                                            BlockState::YELLOW_WALL_BANNER,
                                        ),
                                        BlockKind::LimeBanner => face_wall_block(
                                            face,
                                            BlockState::LIME_BANNER,
                                            BlockState::LIME_WALL_BANNER,
                                        ),
                                        BlockKind::PinkBanner => face_wall_block(
                                            face,
                                            BlockState::PINK_BANNER,
                                            BlockState::PINK_WALL_BANNER,
                                        ),
                                        BlockKind::GrayBanner => face_wall_block(
                                            face,
                                            BlockState::GRAY_BANNER,
                                            BlockState::GRAY_WALL_BANNER,
                                        ),
                                        BlockKind::LightGrayBanner => face_wall_block(
                                            face,
                                            BlockState::LIGHT_GRAY_BANNER,
                                            BlockState::LIGHT_GRAY_WALL_BANNER,
                                        ),
                                        BlockKind::CyanBanner => face_wall_block(
                                            face,
                                            BlockState::CYAN_BANNER,
                                            BlockState::CYAN_WALL_BANNER,
                                        ),
                                        BlockKind::PurpleBanner => face_wall_block(
                                            face,
                                            BlockState::PURPLE_BANNER,
                                            BlockState::PURPLE_WALL_BANNER,
                                        ),
                                        BlockKind::BlueBanner => face_wall_block(
                                            face,
                                            BlockState::BLUE_BANNER,
                                            BlockState::BLUE_WALL_BANNER,
                                        ),
                                        BlockKind::BrownBanner => face_wall_block(
                                            face,
                                            BlockState::BROWN_BANNER,
                                            BlockState::BROWN_WALL_BANNER,
                                        ),
                                        BlockKind::GreenBanner => face_wall_block(
                                            face,
                                            BlockState::GREEN_BANNER,
                                            BlockState::GREEN_WALL_BANNER,
                                        ),
                                        BlockKind::RedBanner => face_wall_block(
                                            face,
                                            BlockState::RED_BANNER,
                                            BlockState::RED_WALL_BANNER,
                                        ),
                                        BlockKind::BlackBanner => face_wall_block(
                                            face,
                                            BlockState::BLACK_BANNER,
                                            BlockState::BLACK_WALL_BANNER,
                                        ),
                                        // Corals
                                        BlockKind::DeadTubeCoralFan => face_wall_block(
                                            face,
                                            BlockState::DEAD_TUBE_CORAL_FAN,
                                            BlockState::DEAD_TUBE_CORAL_WALL_FAN,
                                        ),
                                        BlockKind::DeadBrainCoralFan => face_wall_block(
                                            face,
                                            BlockState::DEAD_BRAIN_CORAL_FAN,
                                            BlockState::DEAD_BRAIN_CORAL_WALL_FAN,
                                        ),
                                        BlockKind::DeadBubbleCoralFan => face_wall_block(
                                            face,
                                            BlockState::DEAD_BUBBLE_CORAL_FAN,
                                            BlockState::DEAD_BUBBLE_CORAL_WALL_FAN,
                                        ),
                                        BlockKind::DeadFireCoralFan => face_wall_block(
                                            face,
                                            BlockState::DEAD_FIRE_CORAL_FAN,
                                            BlockState::DEAD_FIRE_CORAL_WALL_FAN,
                                        ),
                                        BlockKind::DeadHornCoralFan => face_wall_block(
                                            face,
                                            BlockState::DEAD_HORN_CORAL_FAN,
                                            BlockState::DEAD_HORN_CORAL_WALL_FAN,
                                        ),
                                        BlockKind::TubeCoralFan => face_wall_block(
                                            face,
                                            BlockState::TUBE_CORAL_FAN,
                                            BlockState::TUBE_CORAL_WALL_FAN,
                                        ),
                                        BlockKind::BrainCoralFan => face_wall_block(
                                            face,
                                            BlockState::BRAIN_CORAL_FAN,
                                            BlockState::BRAIN_CORAL_WALL_FAN,
                                        ),
                                        BlockKind::BubbleCoralFan => face_wall_block(
                                            face,
                                            BlockState::BUBBLE_CORAL_FAN,
                                            BlockState::BUBBLE_CORAL_WALL_FAN,
                                        ),
                                        BlockKind::FireCoralFan => face_wall_block(
                                            face,
                                            BlockState::FIRE_CORAL_FAN,
                                            BlockState::FIRE_CORAL_WALL_FAN,
                                        ),
                                        BlockKind::HornCoralFan => face_wall_block(
                                            face,
                                            BlockState::HORN_CORAL_FAN,
                                            BlockState::HORN_CORAL_WALL_FAN,
                                        ),
                                        // Logs and stems
                                        BlockKind::OakLog => {
                                            face_directional_block(face, BlockState::OAK_LOG)
                                        }

                                        BlockKind::SpruceLog => {
                                            face_directional_block(face, BlockState::SPRUCE_LOG)
                                        }
                                        BlockKind::BirchLog => {
                                            face_directional_block(face, BlockState::BIRCH_LOG)
                                        }
                                        BlockKind::JungleLog => {
                                            face_directional_block(face, BlockState::JUNGLE_LOG)
                                        }

                                        BlockKind::AcaciaLog => {
                                            face_directional_block(face, BlockState::ACACIA_LOG)
                                        }

                                        BlockKind::DarkOakLog => {
                                            face_directional_block(face, BlockState::DARK_OAK_LOG)
                                        }

                                        BlockKind::MangroveLog => {
                                            face_directional_block(face, BlockState::MANGROVE_LOG)
                                        }

                                        BlockKind::CrimsonStem => {
                                            face_directional_block(face, BlockState::CRIMSON_STEM)
                                        }

                                        BlockKind::WarpedStem => {
                                            face_directional_block(face, BlockState::WARPED_STEM)
                                        }

                                        // Stripped logs
                                        BlockKind::StrippedOakLog => face_directional_block(
                                            face,
                                            BlockState::STRIPPED_OAK_LOG,
                                        ),
                                        BlockKind::StrippedSpruceLog => face_directional_block(
                                            face,
                                            BlockState::STRIPPED_SPRUCE_LOG,
                                        ),
                                        BlockKind::StrippedBirchLog => face_directional_block(
                                            face,
                                            BlockState::STRIPPED_BIRCH_LOG,
                                        ),
                                        BlockKind::StrippedJungleLog => face_directional_block(
                                            face,
                                            BlockState::STRIPPED_JUNGLE_LOG,
                                        ),
                                        BlockKind::StrippedAcaciaLog => face_directional_block(
                                            face,
                                            BlockState::STRIPPED_ACACIA_LOG,
                                        ),
                                        BlockKind::StrippedDarkOakLog => face_directional_block(
                                            face,
                                            BlockState::STRIPPED_DARK_OAK_LOG,
                                        ),
                                        BlockKind::StrippedMangroveLog => face_directional_block(
                                            face,
                                            BlockState::STRIPPED_MANGROVE_LOG,
                                        ),
                                        BlockKind::StrippedCrimsonStem => face_directional_block(
                                            face,
                                            BlockState::STRIPPED_CRIMSON_STEM,
                                        ),
                                        BlockKind::StrippedWarpedStem => face_directional_block(
                                            face,
                                            BlockState::STRIPPED_WARPED_STEM,
                                        ),
                                        kind => BlockState::from_kind(kind),
                                    };

                                    world.chunks.set_block_state(place_at, state);
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

fn face_wall_block(face: BlockFace, normal: BlockState, wall: BlockState) -> BlockState {
    match face {
        BlockFace::Bottom => normal,
        BlockFace::Top => normal,
        BlockFace::North => wall.set(PropName::Facing, PropValue::North),
        BlockFace::South => wall.set(PropName::Facing, PropValue::South),
        BlockFace::West => wall.set(PropName::Facing, PropValue::West),
        BlockFace::East => wall.set(PropName::Facing, PropValue::East),
    }
}

fn face_directional_block(face: BlockFace, block: BlockState) -> BlockState {
    match face {
        BlockFace::Bottom | BlockFace::Top => block.set(PropName::Axis, PropValue::Y),
        BlockFace::North | BlockFace::South => block.set(PropName::Axis, PropValue::Z),
        BlockFace::West | BlockFace::East => block.set(PropName::Axis, PropValue::X),
    }
}
