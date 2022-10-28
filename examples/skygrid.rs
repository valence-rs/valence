use std::net::SocketAddr;
use std::ops::Range;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use rand::seq::SliceRandom;
use rayon::prelude::ParallelIterator;
use valence::async_trait;
use valence::block::BlockState;
use valence::chunk::{Chunk, ChunkPos, UnloadedChunk};
use valence::client::{handle_event_default, GameMode};
use valence::config::{Config, ServerListPing};
use valence::dimension::DimensionId;
use valence::entity::{EntityId, EntityKind};
use valence::player_list::PlayerListId;
use valence::server::{Server, SharedServer, ShutdownResult};
use valence::text::{Color, TextFormat};
use valence::util::chunks_in_view_distance;

const MAX_PLAYERS: usize = 10;
const Y_RANGE: Range<i64> = -64..319;
const BLOCK_SPACING: i64 = 4;

pub fn main() -> ShutdownResult {
    env_logger::Builder::new()
        .filter_module("valence", LevelFilter::Trace)
        .parse_default_env()
        .init();
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

#[async_trait]
impl Config for Game {
    type ServerState = Option<PlayerListId>;
    type ClientState = EntityId;
    type EntityState = ();
    type WorldState = ();
    /// If the chunk should stay loaded at the end of the tick.
    type ChunkState = bool;
    type PlayerListState = ();

    fn max_connections(&self) -> usize {
        // We want status pings to be successful even if the server is full.
        MAX_PLAYERS + 64
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
        server.worlds.insert(DimensionId::default(), ());
        server.state = Some(server.player_lists.insert(()).0);
    }

    fn update(&self, server: &mut Server<Self>) {
        let (world_id, world) = server.worlds.iter_mut().next().unwrap();

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
                client.set_game_mode(GameMode::Creative);
                client.teleport([0.0, 200.0, 0.0], 0.0, 0.0);
                client.set_player_list(server.state.clone());

                if let Some(id) = &server.state {
                    server.player_lists.get_mut(id).insert(
                        client.uuid(),
                        client.username(),
                        client.textures().cloned(),
                        client.game_mode(),
                        0,
                        None,
                    );
                }

                client.send_message("Welcome to SkyGrid!".italic());
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                if let Some(id) = &server.state {
                    server.player_lists.get_mut(id).remove(client.uuid());
                }
                server.entities.remove(client.state);

                return false;
            }

            if let Some(entity) = server.entities.get_mut(client.state) {
                while handle_event_default(client, entity).is_some() {}
            }

            let dist = client.view_distance();
            let p = client.position();

            for pos in chunks_in_view_distance(ChunkPos::at(p.x, p.z), dist) {
                if let Some(chunk) = world.chunks.get_mut(pos) {
                    chunk.state = true;
                } else {
                    world.chunks.insert(pos, UnloadedChunk::default(), true);
                }
            }

            true
        });

        // Remove chunks outside the view distance of players.
        world.chunks.retain(|_, chunk| {
            if chunk.state {
                chunk.state = false;
                true
            } else {
                false
            }
        });

        // Generate chunk data for chunks created this tick.
        world.chunks.par_iter_mut().for_each(|(pos, chunk)| {
            if !chunk.created_this_tick() {
                return;
            }

            for z in 0..16 {
                for x in 0..16 {
                    let block_x = x as i64 + pos.x as i64 * 16;
                    let block_z = z as i64 + pos.z as i64 * 16;

                    for y in (0..chunk.height()).rev() {
                        let b = terrain_column(block_x, y as i64, block_z);
                        chunk.set_block_state(x, y, z, b);
                    }
                }
            }
        });
    }
}

fn terrain_column(x: i64, y: i64, z: i64) -> BlockState {
    if has_terrain_at(x, y, z) {
        *BLOCK_TYPES
            .choose(&mut rand::thread_rng())
            .unwrap_or(&BlockState::STONE)
    } else {
        BlockState::AIR
    }
}

fn has_terrain_at(x: i64, y: i64, z: i64) -> bool {
    Y_RANGE.min().unwrap_or(-64) <= y
        && y <= Y_RANGE.max().unwrap_or(319)
        && x % BLOCK_SPACING == 0
        && y % BLOCK_SPACING == 0
        && z % BLOCK_SPACING == 0
}

const BLOCK_TYPES: [BlockState; 547] = [
    BlockState::STONE,
    BlockState::GRANITE,
    BlockState::POLISHED_GRANITE,
    BlockState::DIORITE,
    BlockState::POLISHED_DIORITE,
    BlockState::ANDESITE,
    BlockState::POLISHED_ANDESITE,
    BlockState::GRASS_BLOCK,
    BlockState::DIRT,
    BlockState::COARSE_DIRT,
    BlockState::PODZOL,
    BlockState::COBBLESTONE,
    BlockState::OAK_PLANKS,
    BlockState::SPRUCE_PLANKS,
    BlockState::BIRCH_PLANKS,
    BlockState::JUNGLE_PLANKS,
    BlockState::ACACIA_PLANKS,
    BlockState::DARK_OAK_PLANKS,
    BlockState::MANGROVE_PLANKS,
    BlockState::BEDROCK,
    BlockState::SAND,
    BlockState::RED_SAND,
    BlockState::GRAVEL,
    BlockState::GOLD_ORE,
    BlockState::DEEPSLATE_GOLD_ORE,
    BlockState::IRON_ORE,
    BlockState::DEEPSLATE_IRON_ORE,
    BlockState::COAL_ORE,
    BlockState::DEEPSLATE_COAL_ORE,
    BlockState::NETHER_GOLD_ORE,
    BlockState::OAK_LOG,
    BlockState::SPRUCE_LOG,
    BlockState::BIRCH_LOG,
    BlockState::JUNGLE_LOG,
    BlockState::ACACIA_LOG,
    BlockState::DARK_OAK_LOG,
    BlockState::MANGROVE_LOG,
    BlockState::MUDDY_MANGROVE_ROOTS,
    BlockState::STRIPPED_SPRUCE_LOG,
    BlockState::STRIPPED_BIRCH_LOG,
    BlockState::STRIPPED_JUNGLE_LOG,
    BlockState::STRIPPED_ACACIA_LOG,
    BlockState::STRIPPED_DARK_OAK_LOG,
    BlockState::STRIPPED_OAK_LOG,
    BlockState::STRIPPED_MANGROVE_LOG,
    BlockState::OAK_WOOD,
    BlockState::SPRUCE_WOOD,
    BlockState::BIRCH_WOOD,
    BlockState::JUNGLE_WOOD,
    BlockState::ACACIA_WOOD,
    BlockState::DARK_OAK_WOOD,
    BlockState::MANGROVE_WOOD,
    BlockState::STRIPPED_OAK_WOOD,
    BlockState::STRIPPED_SPRUCE_WOOD,
    BlockState::STRIPPED_BIRCH_WOOD,
    BlockState::STRIPPED_JUNGLE_WOOD,
    BlockState::STRIPPED_ACACIA_WOOD,
    BlockState::STRIPPED_DARK_OAK_WOOD,
    BlockState::STRIPPED_MANGROVE_WOOD,
    BlockState::SPONGE,
    BlockState::WET_SPONGE,
    BlockState::LAPIS_ORE,
    BlockState::DEEPSLATE_LAPIS_ORE,
    BlockState::LAPIS_BLOCK,
    BlockState::DISPENSER,
    BlockState::SANDSTONE,
    BlockState::CHISELED_SANDSTONE,
    BlockState::CUT_SANDSTONE,
    BlockState::NOTE_BLOCK,
    BlockState::STICKY_PISTON,
    BlockState::PISTON,
    BlockState::PISTON_HEAD,
    BlockState::WHITE_WOOL,
    BlockState::ORANGE_WOOL,
    BlockState::MAGENTA_WOOL,
    BlockState::LIGHT_BLUE_WOOL,
    BlockState::YELLOW_WOOL,
    BlockState::LIME_WOOL,
    BlockState::PINK_WOOL,
    BlockState::GRAY_WOOL,
    BlockState::LIGHT_GRAY_WOOL,
    BlockState::CYAN_WOOL,
    BlockState::PURPLE_WOOL,
    BlockState::BLUE_WOOL,
    BlockState::BROWN_WOOL,
    BlockState::GREEN_WOOL,
    BlockState::RED_WOOL,
    BlockState::BLACK_WOOL,
    BlockState::GOLD_BLOCK,
    BlockState::IRON_BLOCK,
    BlockState::BRICKS,
    BlockState::TNT,
    BlockState::BOOKSHELF,
    BlockState::MOSSY_COBBLESTONE,
    BlockState::OBSIDIAN,
    BlockState::OAK_STAIRS,
    BlockState::CHEST,
    BlockState::DIAMOND_ORE,
    BlockState::DEEPSLATE_DIAMOND_ORE,
    BlockState::DIAMOND_BLOCK,
    BlockState::CRAFTING_TABLE,
    BlockState::FARMLAND,
    BlockState::FURNACE,
    BlockState::COBBLESTONE_STAIRS,
    BlockState::REDSTONE_ORE,
    BlockState::DEEPSLATE_REDSTONE_ORE,
    BlockState::SNOW,
    BlockState::SNOW_BLOCK,
    BlockState::CACTUS,
    BlockState::CLAY,
    BlockState::JUKEBOX,
    BlockState::OAK_FENCE,
    BlockState::PUMPKIN,
    BlockState::NETHERRACK,
    BlockState::SOUL_SAND,
    BlockState::SOUL_SOIL,
    BlockState::BASALT,
    BlockState::POLISHED_BASALT,
    BlockState::GLOWSTONE,
    BlockState::CARVED_PUMPKIN,
    BlockState::JACK_O_LANTERN,
    BlockState::CAKE,
    BlockState::REPEATER,
    BlockState::STONE_BRICKS,
    BlockState::MOSSY_STONE_BRICKS,
    BlockState::CRACKED_STONE_BRICKS,
    BlockState::CHISELED_STONE_BRICKS,
    BlockState::PACKED_MUD,
    BlockState::MUD_BRICKS,
    BlockState::INFESTED_STONE,
    BlockState::INFESTED_COBBLESTONE,
    BlockState::INFESTED_STONE_BRICKS,
    BlockState::INFESTED_MOSSY_STONE_BRICKS,
    BlockState::INFESTED_CRACKED_STONE_BRICKS,
    BlockState::INFESTED_CHISELED_STONE_BRICKS,
    BlockState::BROWN_MUSHROOM_BLOCK,
    BlockState::RED_MUSHROOM_BLOCK,
    BlockState::MUSHROOM_STEM,
    BlockState::MELON,
    BlockState::OAK_FENCE_GATE,
    BlockState::BRICK_STAIRS,
    BlockState::STONE_BRICK_STAIRS,
    BlockState::MUD_BRICK_STAIRS,
    BlockState::MYCELIUM,
    BlockState::NETHER_BRICKS,
    BlockState::NETHER_BRICK_FENCE,
    BlockState::NETHER_BRICK_STAIRS,
    BlockState::ENCHANTING_TABLE,
    BlockState::END_PORTAL_FRAME,
    BlockState::END_STONE,
    BlockState::REDSTONE_LAMP,
    BlockState::SANDSTONE_STAIRS,
    BlockState::EMERALD_ORE,
    BlockState::DEEPSLATE_EMERALD_ORE,
    BlockState::ENDER_CHEST,
    BlockState::EMERALD_BLOCK,
    BlockState::SPRUCE_STAIRS,
    BlockState::BIRCH_STAIRS,
    BlockState::JUNGLE_STAIRS,
    BlockState::COMMAND_BLOCK,
    BlockState::COBBLESTONE_WALL,
    BlockState::MOSSY_COBBLESTONE_WALL,
    BlockState::SKELETON_SKULL,
    BlockState::SKELETON_SKULL,
    BlockState::WITHER_SKELETON_SKULL,
    BlockState::WITHER_SKELETON_SKULL,
    BlockState::ZOMBIE_HEAD,
    BlockState::ZOMBIE_HEAD,
    BlockState::PLAYER_HEAD,
    BlockState::PLAYER_HEAD,
    BlockState::CREEPER_HEAD,
    BlockState::CREEPER_HEAD,
    BlockState::DRAGON_HEAD,
    BlockState::DRAGON_HEAD,
    BlockState::ANVIL,
    BlockState::CHIPPED_ANVIL,
    BlockState::DAMAGED_ANVIL,
    BlockState::TRAPPED_CHEST,
    BlockState::COMPARATOR,
    BlockState::DAYLIGHT_DETECTOR,
    BlockState::REDSTONE_BLOCK,
    BlockState::NETHER_QUARTZ_ORE,
    BlockState::QUARTZ_BLOCK,
    BlockState::CHISELED_QUARTZ_BLOCK,
    BlockState::QUARTZ_PILLAR,
    BlockState::QUARTZ_STAIRS,
    BlockState::DROPPER,
    BlockState::WHITE_TERRACOTTA,
    BlockState::ORANGE_TERRACOTTA,
    BlockState::MAGENTA_TERRACOTTA,
    BlockState::LIGHT_BLUE_TERRACOTTA,
    BlockState::YELLOW_TERRACOTTA,
    BlockState::LIME_TERRACOTTA,
    BlockState::PINK_TERRACOTTA,
    BlockState::GRAY_TERRACOTTA,
    BlockState::LIGHT_GRAY_TERRACOTTA,
    BlockState::CYAN_TERRACOTTA,
    BlockState::PURPLE_TERRACOTTA,
    BlockState::BLUE_TERRACOTTA,
    BlockState::BROWN_TERRACOTTA,
    BlockState::GREEN_TERRACOTTA,
    BlockState::RED_TERRACOTTA,
    BlockState::BLACK_TERRACOTTA,
    BlockState::ACACIA_STAIRS,
    BlockState::DARK_OAK_STAIRS,
    BlockState::MANGROVE_STAIRS,
    BlockState::PRISMARINE,
    BlockState::PRISMARINE_BRICKS,
    BlockState::DARK_PRISMARINE,
    BlockState::PRISMARINE_STAIRS,
    BlockState::PRISMARINE_BRICK_STAIRS,
    BlockState::DARK_PRISMARINE_STAIRS,
    BlockState::PRISMARINE_SLAB,
    BlockState::PRISMARINE_BRICK_SLAB,
    BlockState::DARK_PRISMARINE_SLAB,
    BlockState::SEA_LANTERN,
    BlockState::HAY_BLOCK,
    BlockState::WHITE_CARPET,
    BlockState::ORANGE_CARPET,
    BlockState::MAGENTA_CARPET,
    BlockState::LIGHT_BLUE_CARPET,
    BlockState::YELLOW_CARPET,
    BlockState::LIME_CARPET,
    BlockState::PINK_CARPET,
    BlockState::GRAY_CARPET,
    BlockState::LIGHT_GRAY_CARPET,
    BlockState::CYAN_CARPET,
    BlockState::PURPLE_CARPET,
    BlockState::BLUE_CARPET,
    BlockState::BROWN_CARPET,
    BlockState::GREEN_CARPET,
    BlockState::RED_CARPET,
    BlockState::BLACK_CARPET,
    BlockState::TERRACOTTA,
    BlockState::COAL_BLOCK,
    BlockState::PACKED_ICE,
    BlockState::RED_SANDSTONE,
    BlockState::CHISELED_RED_SANDSTONE,
    BlockState::CUT_RED_SANDSTONE,
    BlockState::RED_SANDSTONE_STAIRS,
    BlockState::OAK_SLAB,
    BlockState::SPRUCE_SLAB,
    BlockState::BIRCH_SLAB,
    BlockState::JUNGLE_SLAB,
    BlockState::ACACIA_SLAB,
    BlockState::DARK_OAK_SLAB,
    BlockState::MANGROVE_SLAB,
    BlockState::STONE_SLAB,
    BlockState::SMOOTH_STONE_SLAB,
    BlockState::SANDSTONE_SLAB,
    BlockState::CUT_SANDSTONE_SLAB,
    BlockState::PETRIFIED_OAK_SLAB,
    BlockState::COBBLESTONE_SLAB,
    BlockState::BRICK_SLAB,
    BlockState::STONE_BRICK_SLAB,
    BlockState::MUD_BRICK_SLAB,
    BlockState::NETHER_BRICK_SLAB,
    BlockState::QUARTZ_SLAB,
    BlockState::RED_SANDSTONE_SLAB,
    BlockState::CUT_RED_SANDSTONE_SLAB,
    BlockState::PURPUR_SLAB,
    BlockState::SMOOTH_STONE,
    BlockState::SMOOTH_SANDSTONE,
    BlockState::SMOOTH_QUARTZ,
    BlockState::SMOOTH_RED_SANDSTONE,
    BlockState::SPRUCE_FENCE_GATE,
    BlockState::BIRCH_FENCE_GATE,
    BlockState::JUNGLE_FENCE_GATE,
    BlockState::ACACIA_FENCE_GATE,
    BlockState::DARK_OAK_FENCE_GATE,
    BlockState::MANGROVE_FENCE_GATE,
    BlockState::SPRUCE_FENCE,
    BlockState::BIRCH_FENCE,
    BlockState::JUNGLE_FENCE,
    BlockState::ACACIA_FENCE,
    BlockState::DARK_OAK_FENCE,
    BlockState::MANGROVE_FENCE,
    BlockState::PURPUR_BLOCK,
    BlockState::PURPUR_PILLAR,
    BlockState::PURPUR_STAIRS,
    BlockState::END_STONE_BRICKS,
    BlockState::DIRT_PATH,
    BlockState::REPEATING_COMMAND_BLOCK,
    BlockState::CHAIN_COMMAND_BLOCK,
    BlockState::MAGMA_BLOCK,
    BlockState::NETHER_WART_BLOCK,
    BlockState::RED_NETHER_BRICKS,
    BlockState::BONE_BLOCK,
    BlockState::OBSERVER,
    BlockState::WHITE_GLAZED_TERRACOTTA,
    BlockState::ORANGE_GLAZED_TERRACOTTA,
    BlockState::MAGENTA_GLAZED_TERRACOTTA,
    BlockState::LIGHT_BLUE_GLAZED_TERRACOTTA,
    BlockState::YELLOW_GLAZED_TERRACOTTA,
    BlockState::LIME_GLAZED_TERRACOTTA,
    BlockState::PINK_GLAZED_TERRACOTTA,
    BlockState::GRAY_GLAZED_TERRACOTTA,
    BlockState::LIGHT_GRAY_GLAZED_TERRACOTTA,
    BlockState::CYAN_GLAZED_TERRACOTTA,
    BlockState::PURPLE_GLAZED_TERRACOTTA,
    BlockState::BLUE_GLAZED_TERRACOTTA,
    BlockState::BROWN_GLAZED_TERRACOTTA,
    BlockState::GREEN_GLAZED_TERRACOTTA,
    BlockState::RED_GLAZED_TERRACOTTA,
    BlockState::BLACK_GLAZED_TERRACOTTA,
    BlockState::WHITE_CONCRETE,
    BlockState::ORANGE_CONCRETE,
    BlockState::MAGENTA_CONCRETE,
    BlockState::LIGHT_BLUE_CONCRETE,
    BlockState::YELLOW_CONCRETE,
    BlockState::LIME_CONCRETE,
    BlockState::PINK_CONCRETE,
    BlockState::GRAY_CONCRETE,
    BlockState::LIGHT_GRAY_CONCRETE,
    BlockState::CYAN_CONCRETE,
    BlockState::PURPLE_CONCRETE,
    BlockState::BLUE_CONCRETE,
    BlockState::BROWN_CONCRETE,
    BlockState::GREEN_CONCRETE,
    BlockState::RED_CONCRETE,
    BlockState::BLACK_CONCRETE,
    BlockState::WHITE_CONCRETE_POWDER,
    BlockState::ORANGE_CONCRETE_POWDER,
    BlockState::MAGENTA_CONCRETE_POWDER,
    BlockState::LIGHT_BLUE_CONCRETE_POWDER,
    BlockState::YELLOW_CONCRETE_POWDER,
    BlockState::LIME_CONCRETE_POWDER,
    BlockState::PINK_CONCRETE_POWDER,
    BlockState::GRAY_CONCRETE_POWDER,
    BlockState::LIGHT_GRAY_CONCRETE_POWDER,
    BlockState::CYAN_CONCRETE_POWDER,
    BlockState::PURPLE_CONCRETE_POWDER,
    BlockState::BLUE_CONCRETE_POWDER,
    BlockState::BROWN_CONCRETE_POWDER,
    BlockState::GREEN_CONCRETE_POWDER,
    BlockState::RED_CONCRETE_POWDER,
    BlockState::BLACK_CONCRETE_POWDER,
    BlockState::DRIED_KELP_BLOCK,
    BlockState::DEAD_TUBE_CORAL_BLOCK,
    BlockState::DEAD_BRAIN_CORAL_BLOCK,
    BlockState::DEAD_BUBBLE_CORAL_BLOCK,
    BlockState::DEAD_FIRE_CORAL_BLOCK,
    BlockState::DEAD_HORN_CORAL_BLOCK,
    BlockState::TUBE_CORAL_BLOCK,
    BlockState::BRAIN_CORAL_BLOCK,
    BlockState::BUBBLE_CORAL_BLOCK,
    BlockState::FIRE_CORAL_BLOCK,
    BlockState::HORN_CORAL_BLOCK,
    BlockState::BLUE_ICE,
    BlockState::POLISHED_GRANITE_STAIRS,
    BlockState::SMOOTH_RED_SANDSTONE_STAIRS,
    BlockState::MOSSY_STONE_BRICK_STAIRS,
    BlockState::POLISHED_DIORITE_STAIRS,
    BlockState::MOSSY_COBBLESTONE_STAIRS,
    BlockState::END_STONE_BRICK_STAIRS,
    BlockState::STONE_STAIRS,
    BlockState::SMOOTH_SANDSTONE_STAIRS,
    BlockState::SMOOTH_QUARTZ_STAIRS,
    BlockState::GRANITE_STAIRS,
    BlockState::ANDESITE_STAIRS,
    BlockState::RED_NETHER_BRICK_STAIRS,
    BlockState::POLISHED_ANDESITE_STAIRS,
    BlockState::DIORITE_STAIRS,
    BlockState::POLISHED_GRANITE_SLAB,
    BlockState::SMOOTH_RED_SANDSTONE_SLAB,
    BlockState::MOSSY_STONE_BRICK_SLAB,
    BlockState::POLISHED_DIORITE_SLAB,
    BlockState::MOSSY_COBBLESTONE_SLAB,
    BlockState::END_STONE_BRICK_SLAB,
    BlockState::SMOOTH_SANDSTONE_SLAB,
    BlockState::SMOOTH_QUARTZ_SLAB,
    BlockState::GRANITE_SLAB,
    BlockState::ANDESITE_SLAB,
    BlockState::RED_NETHER_BRICK_SLAB,
    BlockState::POLISHED_ANDESITE_SLAB,
    BlockState::DIORITE_SLAB,
    BlockState::BRICK_WALL,
    BlockState::PRISMARINE_WALL,
    BlockState::RED_SANDSTONE_WALL,
    BlockState::MOSSY_STONE_BRICK_WALL,
    BlockState::GRANITE_WALL,
    BlockState::STONE_BRICK_WALL,
    BlockState::MUD_BRICK_WALL,
    BlockState::NETHER_BRICK_WALL,
    BlockState::ANDESITE_WALL,
    BlockState::RED_NETHER_BRICK_WALL,
    BlockState::SANDSTONE_WALL,
    BlockState::END_STONE_BRICK_WALL,
    BlockState::DIORITE_WALL,
    BlockState::LOOM,
    BlockState::BARREL,
    BlockState::SMOKER,
    BlockState::BLAST_FURNACE,
    BlockState::CARTOGRAPHY_TABLE,
    BlockState::FLETCHING_TABLE,
    BlockState::GRINDSTONE,
    BlockState::LECTERN,
    BlockState::SMITHING_TABLE,
    BlockState::STONECUTTER,
    BlockState::BELL,
    BlockState::WARPED_STEM,
    BlockState::STRIPPED_WARPED_STEM,
    BlockState::WARPED_HYPHAE,
    BlockState::STRIPPED_WARPED_HYPHAE,
    BlockState::WARPED_NYLIUM,
    BlockState::WARPED_WART_BLOCK,
    BlockState::CRIMSON_STEM,
    BlockState::STRIPPED_CRIMSON_STEM,
    BlockState::CRIMSON_HYPHAE,
    BlockState::STRIPPED_CRIMSON_HYPHAE,
    BlockState::CRIMSON_NYLIUM,
    BlockState::SHROOMLIGHT,
    BlockState::CRIMSON_PLANKS,
    BlockState::WARPED_PLANKS,
    BlockState::CRIMSON_SLAB,
    BlockState::WARPED_SLAB,
    BlockState::CRIMSON_FENCE,
    BlockState::WARPED_FENCE,
    BlockState::CRIMSON_FENCE_GATE,
    BlockState::WARPED_FENCE_GATE,
    BlockState::CRIMSON_STAIRS,
    BlockState::WARPED_STAIRS,
    BlockState::STRUCTURE_BLOCK,
    BlockState::JIGSAW,
    BlockState::COMPOSTER,
    BlockState::TARGET,
    BlockState::BEE_NEST,
    BlockState::BEEHIVE,
    BlockState::HONEYCOMB_BLOCK,
    BlockState::NETHERITE_BLOCK,
    BlockState::ANCIENT_DEBRIS,
    BlockState::CRYING_OBSIDIAN,
    BlockState::RESPAWN_ANCHOR,
    BlockState::LODESTONE,
    BlockState::BLACKSTONE,
    BlockState::BLACKSTONE_STAIRS,
    BlockState::BLACKSTONE_WALL,
    BlockState::BLACKSTONE_SLAB,
    BlockState::POLISHED_BLACKSTONE,
    BlockState::POLISHED_BLACKSTONE_BRICKS,
    BlockState::CRACKED_POLISHED_BLACKSTONE_BRICKS,
    BlockState::CHISELED_POLISHED_BLACKSTONE,
    BlockState::POLISHED_BLACKSTONE_BRICK_SLAB,
    BlockState::POLISHED_BLACKSTONE_BRICK_STAIRS,
    BlockState::POLISHED_BLACKSTONE_BRICK_WALL,
    BlockState::GILDED_BLACKSTONE,
    BlockState::POLISHED_BLACKSTONE_STAIRS,
    BlockState::POLISHED_BLACKSTONE_SLAB,
    BlockState::POLISHED_BLACKSTONE_WALL,
    BlockState::CHISELED_NETHER_BRICKS,
    BlockState::CRACKED_NETHER_BRICKS,
    BlockState::QUARTZ_BRICKS,
    BlockState::CANDLE_CAKE,
    BlockState::WHITE_CANDLE_CAKE,
    BlockState::ORANGE_CANDLE_CAKE,
    BlockState::MAGENTA_CANDLE_CAKE,
    BlockState::LIGHT_BLUE_CANDLE_CAKE,
    BlockState::YELLOW_CANDLE_CAKE,
    BlockState::LIME_CANDLE_CAKE,
    BlockState::PINK_CANDLE_CAKE,
    BlockState::GRAY_CANDLE_CAKE,
    BlockState::LIGHT_GRAY_CANDLE_CAKE,
    BlockState::CYAN_CANDLE_CAKE,
    BlockState::PURPLE_CANDLE_CAKE,
    BlockState::BLUE_CANDLE_CAKE,
    BlockState::BROWN_CANDLE_CAKE,
    BlockState::GREEN_CANDLE_CAKE,
    BlockState::RED_CANDLE_CAKE,
    BlockState::BLACK_CANDLE_CAKE,
    BlockState::AMETHYST_BLOCK,
    BlockState::BUDDING_AMETHYST,
    BlockState::TUFF,
    BlockState::CALCITE,
    BlockState::POWDER_SNOW,
    BlockState::SCULK_SENSOR,
    BlockState::SCULK,
    BlockState::SCULK_CATALYST,
    BlockState::SCULK_SHRIEKER,
    BlockState::OXIDIZED_COPPER,
    BlockState::WEATHERED_COPPER,
    BlockState::EXPOSED_COPPER,
    BlockState::COPPER_BLOCK,
    BlockState::COPPER_ORE,
    BlockState::DEEPSLATE_COPPER_ORE,
    BlockState::OXIDIZED_CUT_COPPER,
    BlockState::WEATHERED_CUT_COPPER,
    BlockState::EXPOSED_CUT_COPPER,
    BlockState::CUT_COPPER,
    BlockState::OXIDIZED_CUT_COPPER_STAIRS,
    BlockState::WEATHERED_CUT_COPPER_STAIRS,
    BlockState::EXPOSED_CUT_COPPER_STAIRS,
    BlockState::CUT_COPPER_STAIRS,
    BlockState::OXIDIZED_CUT_COPPER_SLAB,
    BlockState::WEATHERED_CUT_COPPER_SLAB,
    BlockState::EXPOSED_CUT_COPPER_SLAB,
    BlockState::CUT_COPPER_SLAB,
    BlockState::WAXED_COPPER_BLOCK,
    BlockState::WAXED_WEATHERED_COPPER,
    BlockState::WAXED_EXPOSED_COPPER,
    BlockState::WAXED_OXIDIZED_COPPER,
    BlockState::WAXED_OXIDIZED_CUT_COPPER,
    BlockState::WAXED_WEATHERED_CUT_COPPER,
    BlockState::WAXED_EXPOSED_CUT_COPPER,
    BlockState::WAXED_CUT_COPPER,
    BlockState::WAXED_OXIDIZED_CUT_COPPER_STAIRS,
    BlockState::WAXED_WEATHERED_CUT_COPPER_STAIRS,
    BlockState::WAXED_EXPOSED_CUT_COPPER_STAIRS,
    BlockState::WAXED_CUT_COPPER_STAIRS,
    BlockState::WAXED_OXIDIZED_CUT_COPPER_SLAB,
    BlockState::WAXED_WEATHERED_CUT_COPPER_SLAB,
    BlockState::WAXED_EXPOSED_CUT_COPPER_SLAB,
    BlockState::WAXED_CUT_COPPER_SLAB,
    BlockState::DRIPSTONE_BLOCK,
    BlockState::MOSS_CARPET,
    BlockState::MOSS_BLOCK,
    BlockState::BIG_DRIPLEAF,
    BlockState::ROOTED_DIRT,
    BlockState::MUD,
    BlockState::DEEPSLATE,
    BlockState::COBBLED_DEEPSLATE,
    BlockState::COBBLED_DEEPSLATE_STAIRS,
    BlockState::COBBLED_DEEPSLATE_SLAB,
    BlockState::COBBLED_DEEPSLATE_WALL,
    BlockState::POLISHED_DEEPSLATE,
    BlockState::POLISHED_DEEPSLATE_STAIRS,
    BlockState::POLISHED_DEEPSLATE_SLAB,
    BlockState::POLISHED_DEEPSLATE_WALL,
    BlockState::DEEPSLATE_TILES,
    BlockState::DEEPSLATE_TILE_STAIRS,
    BlockState::DEEPSLATE_TILE_SLAB,
    BlockState::DEEPSLATE_TILE_WALL,
    BlockState::DEEPSLATE_BRICKS,
    BlockState::DEEPSLATE_BRICK_STAIRS,
    BlockState::DEEPSLATE_BRICK_SLAB,
    BlockState::DEEPSLATE_BRICK_WALL,
    BlockState::CHISELED_DEEPSLATE,
    BlockState::CRACKED_DEEPSLATE_BRICKS,
    BlockState::CRACKED_DEEPSLATE_TILES,
    BlockState::INFESTED_DEEPSLATE,
    BlockState::SMOOTH_BASALT,
    BlockState::RAW_IRON_BLOCK,
    BlockState::RAW_COPPER_BLOCK,
    BlockState::RAW_GOLD_BLOCK,
    BlockState::OCHRE_FROGLIGHT,
    BlockState::VERDANT_FROGLIGHT,
    BlockState::PEARLESCENT_FROGLIGHT,
    BlockState::REINFORCED_DEEPSLATE,
];
