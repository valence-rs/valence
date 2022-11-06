use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use valence::client::SetTitleAnimationTimes;
use valence::particle::ParticleType;
use valence::prelude::*;
use vek::Rgb;

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
            particle_list: create_particle_vec(),
            particle_index: 0,
        },
    )
}

struct Game {
    player_count: AtomicUsize,
}

struct ServerState {
    player_list: Option<PlayerListId>,
    particle_list: Vec<ParticleType>,
    particle_index: usize,
}

const MAX_PLAYERS: usize = 10;

const SPAWN_POS: BlockPos = BlockPos::new(0, 100, -25);

#[async_trait]
impl Config for Game {
    type ServerState = ServerState;
    type ClientState = EntityId;
    type EntityState = ();
    type WorldState = ();
    type ChunkState = ();
    type PlayerListState = ();

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
        let (_world_id, world) = server.worlds.insert(DimensionId::default(), ());
        server.state.player_list = Some(server.player_lists.insert(()).0);

        let size = 5;
        for z in -size..size {
            for x in -size..size {
                world.chunks.insert([x, z], UnloadedChunk::default(), ());
            }
        }

        world.chunks.set_block_state(SPAWN_POS, BlockState::BEDROCK);
    }

    fn update(&self, server: &mut Server<Self>) {
        let (world_id, _) = server.worlds.iter_mut().next().expect("missing world");

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
                client.teleport(
                    [
                        SPAWN_POS.x as f64 + 0.5,
                        SPAWN_POS.y as f64 + 1.0,
                        SPAWN_POS.z as f64 + 0.5,
                    ],
                    0.0,
                    0.0,
                );
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
            }

            if client.is_disconnected() {
                self.player_count.fetch_sub(1, Ordering::SeqCst);
                if let Some(id) = &server.state.player_list {
                    server.player_lists.get_mut(id).remove(client.uuid());
                }
                server.entities.remove(client.state);

                return false;
            }

            let entity = server
                .entities
                .get_mut(client.state)
                .expect("missing player entity");

            while handle_event_default(client, entity).is_some() {}

            true
        });

        // TODO add a way to speed up particle cycle, for testing
        if !server.clients.is_empty() && server.shared.current_tick() % 30 == 0 {
            if server.state.particle_index == server.state.particle_list.len() {
                server.state.particle_index = 0;
            }
            let pos = Vec3::new(0.0, 100.0, -10.0);
            let offset = Vec3::new(0.5, 0.5, 0.5);
            let particle_type: &ParticleType = server
                .state
                .particle_list
                .get(server.state.particle_index)
                .expect("Invalid index to particle list");
            println!("Current particle: {}", particle_type.name());
            server.clients.iter_mut().for_each(|(_cid, client)| {
                client.set_title(
                    "",
                    particle_type.name().to_string().bold(),
                    SetTitleAnimationTimes {
                        fade_in: 0,
                        stay: 100,
                        fade_out: 2,
                    },
                );
                client.play_particle(particle_type.clone(), pos, offset, 0.1, 100, true);
            });
            server.state.particle_index += 1;
        }
    }
}

#[inline]
fn create_particle_vec() -> Vec<ParticleType> {
    vec![
        ParticleType::AmbientEntityEffect,
        ParticleType::AngryVillager,
        ParticleType::Block(BlockState::OAK_PLANKS),
        ParticleType::BlockMarker(BlockState::GOLD_BLOCK),
        ParticleType::Bubble,
        ParticleType::Cloud,
        ParticleType::Crit,
        ParticleType::DamageIndicator,
        ParticleType::DragonBreath,
        ParticleType::DrippingLava,
        ParticleType::FallingLava,
        ParticleType::LandingLava,
        ParticleType::DrippingWater,
        ParticleType::FallingWater,
        ParticleType::Dust {
            rgb: Rgb::new(1.0, 1.0, 0.0),
            scale: 2.0,
        },
        ParticleType::DustColorTransition {
            from_rgb: Rgb::new(1.0, 0.0, 0.0),
            scale: 2.0,
            to_rgb: Rgb::new(0.0, 1.0, 0.0),
        },
        ParticleType::Effect,
        ParticleType::ElderGuardian,
        ParticleType::EnchantedHit,
        ParticleType::Enchant,
        ParticleType::EndRod,
        ParticleType::EntityEffect,
        ParticleType::ExplosionEmitter,
        ParticleType::Explosion,
        ParticleType::SonicBoom,
        ParticleType::FallingDust(BlockState::RED_SAND),
        ParticleType::Firework,
        ParticleType::Fishing,
        ParticleType::Flame,
        ParticleType::SculkSoul,
        ParticleType::SculkCharge { roll: 1.0 },
        ParticleType::SculkChargePop,
        ParticleType::SoulFireFlame,
        ParticleType::Soul,
        ParticleType::Flash,
        ParticleType::HappyVillager,
        ParticleType::Composter,
        ParticleType::Heart,
        ParticleType::InstantEffect,
        ParticleType::VibrationBlock {
            block_pos: SPAWN_POS,
            ticks: 50,
        },
        ParticleType::VibrationEntity {
            entity_id: 0,
            entity_eye_height: 1.0,
            ticks: 50,
        },
        ParticleType::ItemSlime,
        ParticleType::ItemSnowball,
        ParticleType::LargeSmoke,
        ParticleType::Lava,
        ParticleType::Mycelium,
        ParticleType::Note,
        ParticleType::Poof,
        ParticleType::Portal,
        ParticleType::Rain,
        ParticleType::Smoke,
        ParticleType::Sneeze,
        ParticleType::Spit,
        ParticleType::SquidInk,
        ParticleType::SweepAttack,
        ParticleType::TotemOfUndying,
        ParticleType::Underwater,
        ParticleType::Splash,
        ParticleType::Witch,
        ParticleType::BubblePop,
        ParticleType::CurrentDown,
        ParticleType::BubbleColumnUp,
        ParticleType::Nautilus,
        ParticleType::Dolphin,
        ParticleType::CampfireCosySmoke,
        ParticleType::CampfireSignalSmoke,
        ParticleType::DrippingHoney,
        ParticleType::FallingHoney,
        ParticleType::LandingHoney,
        ParticleType::FallingNectar,
        ParticleType::FallingSporeBlossom,
        ParticleType::Ash,
        ParticleType::CrimsonSpore,
        ParticleType::WarpedSpore,
        ParticleType::SporeBlossomAir,
        ParticleType::DrippingObsidianTear,
        ParticleType::FallingObsidianTear,
        ParticleType::LandingObsidianTear,
        ParticleType::ReversePortal,
        ParticleType::WhiteAsh,
        ParticleType::SmallFlame,
        ParticleType::Snowflake,
        ParticleType::DrippingDripstoneLava,
        ParticleType::FallingDripstoneLava,
        ParticleType::DrippingDripstoneWater,
        ParticleType::FallingDripstoneWater,
        ParticleType::GlowSquidInk,
        ParticleType::Glow,
        ParticleType::WaxOn,
        ParticleType::WaxOff,
        ParticleType::ElectricSpark,
        ParticleType::Scrape,
    ]
}
