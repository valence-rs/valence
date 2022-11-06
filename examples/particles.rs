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
            particle_list: Vec::new(),
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
        add_all_particles(server);
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

        if !server.clients.is_empty() && server.shared.current_tick() % 30 == 0 {
            if server.state.particle_index == server.state.particle_list.len() {
                server.state.particle_index = 0;
            }
            let pos = Vec3::new(0.0, 100.0, -10.0);
            let offset = Vec3::new(0.5, 0.5, 0.5);
            let opt: Option<&ParticleType> =
                server.state.particle_list.get(server.state.particle_index);
            match opt {
                None => {
                    unreachable!("Invalid index to particle list");
                }
                Some(particle_type) => {
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
                }
            }
            server.state.particle_index += 1;
        }
    }
}

fn add_all_particles(server: &mut Server<Game>) {
    fn push_particle(server: &mut Server<Game>, particle_type: ParticleType) {
        server.state.particle_list.push(particle_type);
    }
    push_particle(server, ParticleType::AmbientEntityEffect);
    push_particle(server, ParticleType::AngryVillager);
    push_particle(server, ParticleType::Block(BlockState::OAK_PLANKS));
    push_particle(server, ParticleType::BlockMarker(BlockState::GOLD_BLOCK));
    push_particle(server, ParticleType::Bubble);
    push_particle(server, ParticleType::Cloud);
    push_particle(server, ParticleType::Crit);
    push_particle(server, ParticleType::DamageIndicator);
    push_particle(server, ParticleType::DragonBreath);
    push_particle(server, ParticleType::DrippingLava);
    push_particle(server, ParticleType::FallingLava);
    push_particle(server, ParticleType::LandingLava);
    push_particle(server, ParticleType::DrippingWater);
    push_particle(server, ParticleType::FallingWater);
    push_particle(
        server,
        ParticleType::Dust {
            rgb: Rgb::new(1.0, 1.0, 0.0),
            scale: 2.0,
        },
    );
    push_particle(
        server,
        ParticleType::DustColorTransition {
            from_rgb: Rgb::new(1.0, 0.0, 0.0),
            scale: 2.0,
            to_rgb: Rgb::new(0.0, 1.0, 0.0),
        },
    );
    push_particle(server, ParticleType::Effect);
    push_particle(server, ParticleType::ElderGuardian);
    push_particle(server, ParticleType::EnchantedHit);
    push_particle(server, ParticleType::Enchant);
    push_particle(server, ParticleType::EndRod);
    push_particle(server, ParticleType::EntityEffect);
    push_particle(server, ParticleType::ExplosionEmitter);
    push_particle(server, ParticleType::Explosion);
    push_particle(server, ParticleType::SonicBoom);
    push_particle(server, ParticleType::FallingDust(BlockState::RED_SAND));
    push_particle(server, ParticleType::Firework);
    push_particle(server, ParticleType::Fishing);
    push_particle(server, ParticleType::Flame);
    push_particle(server, ParticleType::SculkSoul);
    push_particle(server, ParticleType::SculkCharge { roll: 1.0 });
    push_particle(server, ParticleType::SculkChargePop);
    push_particle(server, ParticleType::SoulFireFlame);
    push_particle(server, ParticleType::Soul);
    push_particle(server, ParticleType::Flash);
    push_particle(server, ParticleType::HappyVillager);
    push_particle(server, ParticleType::Composter);
    push_particle(server, ParticleType::Heart);
    push_particle(server, ParticleType::InstantEffect);
    push_particle(
        server,
        ParticleType::VibrationBlock {
            block_pos: SPAWN_POS,
            ticks: 50,
        },
    );
    push_particle(
        server,
        ParticleType::VibrationEntity {
            entity_id: 0,
            entity_eye_height: 1.0,
            ticks: 50,
        },
    );
    push_particle(server, ParticleType::ItemSlime);
    push_particle(server, ParticleType::ItemSnowball);
    push_particle(server, ParticleType::LargeSmoke);
    push_particle(server, ParticleType::Lava);
    push_particle(server, ParticleType::Mycelium);
    push_particle(server, ParticleType::Note);
    push_particle(server, ParticleType::Poof);
    push_particle(server, ParticleType::Portal);
    push_particle(server, ParticleType::Rain);
    push_particle(server, ParticleType::Smoke);
    push_particle(server, ParticleType::Sneeze);
    push_particle(server, ParticleType::Spit);
    push_particle(server, ParticleType::SquidInk);
    push_particle(server, ParticleType::SweepAttack);
    push_particle(server, ParticleType::TotemOfUndying);
    push_particle(server, ParticleType::Underwater);
    push_particle(server, ParticleType::Splash);
    push_particle(server, ParticleType::Witch);
    push_particle(server, ParticleType::BubblePop);
    push_particle(server, ParticleType::CurrentDown);
    push_particle(server, ParticleType::BubbleColumnUp);
    push_particle(server, ParticleType::Nautilus);
    push_particle(server, ParticleType::Dolphin);
    push_particle(server, ParticleType::CampfireCosySmoke);
    push_particle(server, ParticleType::CampfireSignalSmoke);
    push_particle(server, ParticleType::DrippingHoney);
    push_particle(server, ParticleType::FallingHoney);
    push_particle(server, ParticleType::LandingHoney);
    push_particle(server, ParticleType::FallingNectar);
    push_particle(server, ParticleType::FallingSporeBlossom);
    push_particle(server, ParticleType::Ash);
    push_particle(server, ParticleType::CrimsonSpore);
    push_particle(server, ParticleType::WarpedSpore);
    push_particle(server, ParticleType::SporeBlossomAir);
    push_particle(server, ParticleType::DrippingObsidianTear);
    push_particle(server, ParticleType::FallingObsidianTear);
    push_particle(server, ParticleType::LandingObsidianTear);
    push_particle(server, ParticleType::ReversePortal);
    push_particle(server, ParticleType::WhiteAsh);
    push_particle(server, ParticleType::SmallFlame);
    push_particle(server, ParticleType::Snowflake);
    push_particle(server, ParticleType::DrippingDripstoneLava);
    push_particle(server, ParticleType::FallingDripstoneLava);
    push_particle(server, ParticleType::DrippingDripstoneWater);
    push_particle(server, ParticleType::FallingDripstoneWater);
    push_particle(server, ParticleType::GlowSquidInk);
    push_particle(server, ParticleType::Glow);
    push_particle(server, ParticleType::WaxOn);
    push_particle(server, ParticleType::WaxOff);
    push_particle(server, ParticleType::ElectricSpark);
    push_particle(server, ParticleType::Scrape);
}
