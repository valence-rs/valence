use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use valence::client::SetTitleAnimationTimes;
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
    particle_list: Vec<Particle>,
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
            let particle_type: &Particle = server
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
fn create_particle_vec() -> Vec<Particle> {
    vec![
        Particle::AmbientEntityEffect,
        Particle::AngryVillager,
        Particle::Block(BlockState::OAK_PLANKS),
        Particle::BlockMarker(BlockState::GOLD_BLOCK),
        Particle::Bubble,
        Particle::Cloud,
        Particle::Crit,
        Particle::DamageIndicator,
        Particle::DragonBreath,
        Particle::DrippingLava,
        Particle::FallingLava,
        Particle::LandingLava,
        Particle::DrippingWater,
        Particle::FallingWater,
        Particle::Dust {
            rgb: Rgb::new(1.0, 1.0, 0.0),
            scale: 2.0,
        },
        Particle::DustColorTransition {
            from_rgb: Rgb::new(1.0, 0.0, 0.0),
            scale: 2.0,
            to_rgb: Rgb::new(0.0, 1.0, 0.0),
        },
        Particle::Effect,
        Particle::ElderGuardian,
        Particle::EnchantedHit,
        Particle::Enchant,
        Particle::EndRod,
        Particle::EntityEffect,
        Particle::ExplosionEmitter,
        Particle::Explosion,
        Particle::SonicBoom,
        Particle::FallingDust(BlockState::RED_SAND),
        Particle::Firework,
        Particle::Fishing,
        Particle::Flame,
        Particle::SculkSoul,
        Particle::SculkCharge { roll: 1.0 },
        Particle::SculkChargePop,
        Particle::SoulFireFlame,
        Particle::Soul,
        Particle::Flash,
        Particle::HappyVillager,
        Particle::Composter,
        Particle::Heart,
        Particle::InstantEffect,
        Particle::Item(None),
        Particle::Item(Some(ItemStack::new(ItemKind::IronPickaxe, 1, None))),
        Particle::VibrationBlock {
            block_pos: SPAWN_POS,
            ticks: 50,
        },
        Particle::VibrationEntity {
            entity_id: 0,
            entity_eye_height: 1.0,
            ticks: 50,
        },
        Particle::ItemSlime,
        Particle::ItemSnowball,
        Particle::LargeSmoke,
        Particle::Lava,
        Particle::Mycelium,
        Particle::Note,
        Particle::Poof,
        Particle::Portal,
        Particle::Rain,
        Particle::Smoke,
        Particle::Sneeze,
        Particle::Spit,
        Particle::SquidInk,
        Particle::SweepAttack,
        Particle::TotemOfUndying,
        Particle::Underwater,
        Particle::Splash,
        Particle::Witch,
        Particle::BubblePop,
        Particle::CurrentDown,
        Particle::BubbleColumnUp,
        Particle::Nautilus,
        Particle::Dolphin,
        Particle::CampfireCosySmoke,
        Particle::CampfireSignalSmoke,
        Particle::DrippingHoney,
        Particle::FallingHoney,
        Particle::LandingHoney,
        Particle::FallingNectar,
        Particle::FallingSporeBlossom,
        Particle::Ash,
        Particle::CrimsonSpore,
        Particle::WarpedSpore,
        Particle::SporeBlossomAir,
        Particle::DrippingObsidianTear,
        Particle::FallingObsidianTear,
        Particle::LandingObsidianTear,
        Particle::ReversePortal,
        Particle::WhiteAsh,
        Particle::SmallFlame,
        Particle::Snowflake,
        Particle::DrippingDripstoneLava,
        Particle::FallingDripstoneLava,
        Particle::DrippingDripstoneWater,
        Particle::FallingDripstoneWater,
        Particle::GlowSquidInk,
        Particle::Glow,
        Particle::WaxOn,
        Particle::WaxOff,
        Particle::ElectricSpark,
        Particle::Scrape,
    ]
}
