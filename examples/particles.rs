use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::LevelFilter;
use valence::client::SetTitleAnimationTimes;
use valence::particle::ParticleType;
use valence::prelude::*;
use valence::protocol::VarInt;

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
    fn add_particle_to_list(pt: ParticleType) {
        server.state.particle_list.push(pt);
    }
    add_particle_to_list(ParticleType::AmbientEntityEffect);
    add_particle_to_list(ParticleType::AngryVillager);
    add_particle_to_list(ParticleType::Block(BlockState::OAK_PLANKS));
    add_particle_to_list(ParticleType::BlockMarker(BlockState::GOLD_BLOCK));
    add_particle_to_list(ParticleType::Bubble);
    add_particle_to_list(ParticleType::Cloud);
    add_particle_to_list(ParticleType::Crit);
    add_particle_to_list(ParticleType::DamageIndicator);
    add_particle_to_list(ParticleType::DragonBreath);
    add_particle_to_list(ParticleType::DrippingLava);
    add_particle_to_list(ParticleType::FallingLava);
    add_particle_to_list(ParticleType::LandingLava);
    add_particle_to_list(ParticleType::DrippingWater);
    add_particle_to_list(ParticleType::FallingWater);
    add_particle_to_list(ParticleType::Dust {
        rgb: Vec3::new(1.0, 1.0, 0.0),
        scale: 2.0,
    });
    add_particle_to_list(ParticleType::DustColorTransition {
        from_rgb: Vec3::new(1.0, 0.0, 0.0),
        scale: 2.0,
        to_rgb: Vec3::new(0.0, 1.0, 0.0),
    });
    add_particle_to_list(ParticleType::Effect);
    add_particle_to_list(ParticleType::ElderGuardian);
    add_particle_to_list(ParticleType::EnchantedHit);
    add_particle_to_list(ParticleType::Enchant);
    add_particle_to_list(ParticleType::EndRod);
    add_particle_to_list(ParticleType::EntityEffect);
    add_particle_to_list(ParticleType::ExplosionEmitter);
    add_particle_to_list(ParticleType::Explosion);
    add_particle_to_list(ParticleType::SonicBoom);
    add_particle_to_list(ParticleType::FallingDust(BlockState::RED_SAND));
    add_particle_to_list(ParticleType::Firework);
    add_particle_to_list(ParticleType::Fishing);
    add_particle_to_list(ParticleType::Flame);
    add_particle_to_list(ParticleType::SculkSoul);
    add_particle_to_list(ParticleType::SculkCharge { roll: 1.0 });
    add_particle_to_list(ParticleType::SculkChargePop);
    add_particle_to_list(ParticleType::SoulFireFlame);
    add_particle_to_list(ParticleType::Soul);
    add_particle_to_list(ParticleType::Flash);
    add_particle_to_list(ParticleType::HappyVillager);
    add_particle_to_list(ParticleType::Composter);
    add_particle_to_list(ParticleType::Heart);
    add_particle_to_list(ParticleType::InstantEffect);
    add_particle_to_list(ParticleType::VibrationBlock {
        block_pos: SPAWN_POS,
        ticks: VarInt(50),
    });
    add_particle_to_list(ParticleType::VibrationEntity {
        entity_id: VarInt(0),
        entity_eye_height: 1.0,
        ticks: VarInt(50),
    });
    add_particle_to_list(ParticleType::ItemSlime);
    add_particle_to_list(ParticleType::ItemSnowball);
    add_particle_to_list(ParticleType::LargeSmoke);
    add_particle_to_list(ParticleType::Lava);
    add_particle_to_list(ParticleType::Mycelium);
    add_particle_to_list(ParticleType::Note);
    add_particle_to_list(ParticleType::Poof);
    add_particle_to_list(ParticleType::Portal);
    add_particle_to_list(ParticleType::Rain);
    add_particle_to_list(ParticleType::Smoke);
    add_particle_to_list(ParticleType::Sneeze);
    add_particle_to_list(ParticleType::Spit);
    add_particle_to_list(ParticleType::SquidInk);
    add_particle_to_list(ParticleType::SweepAttack);
    add_particle_to_list(ParticleType::TotemOfUndying);
    add_particle_to_list(ParticleType::Underwater);
    add_particle_to_list(ParticleType::Splash);
    add_particle_to_list(ParticleType::Witch);
    add_particle_to_list(ParticleType::BubblePop);
    add_particle_to_list(ParticleType::CurrentDown);
    add_particle_to_list(ParticleType::BubbleColumnUp);
    add_particle_to_list(ParticleType::Nautilus);
    add_particle_to_list(ParticleType::Dolphin);
    add_particle_to_list(ParticleType::CampfireCosySmoke);
    add_particle_to_list(ParticleType::CampfireSignalSmoke);
    add_particle_to_list(ParticleType::DrippingHoney);
    add_particle_to_list(ParticleType::FallingHoney);
    add_particle_to_list(ParticleType::LandingHoney);
    add_particle_to_list(ParticleType::FallingNectar);
    add_particle_to_list(ParticleType::FallingSporeBlossom);
    add_particle_to_list(ParticleType::Ash);
    add_particle_to_list(ParticleType::CrimsonSpore);
    add_particle_to_list(ParticleType::WarpedSpore);
    add_particle_to_list(ParticleType::SporeBlossomAir);
    add_particle_to_list(ParticleType::DrippingObsidianTear);
    add_particle_to_list(ParticleType::FallingObsidianTear);
    add_particle_to_list(ParticleType::LandingObsidianTear);
    add_particle_to_list(ParticleType::ReversePortal);
    add_particle_to_list(ParticleType::WhiteAsh);
    add_particle_to_list(ParticleType::SmallFlame);
    add_particle_to_list(ParticleType::Snowflake);
    add_particle_to_list(ParticleType::DrippingDripstoneLava);
    add_particle_to_list(ParticleType::FallingDripstoneLava);
    add_particle_to_list(ParticleType::DrippingDripstoneWater);
    add_particle_to_list(ParticleType::FallingDripstoneWater);
    add_particle_to_list(ParticleType::GlowSquidInk);
    add_particle_to_list(ParticleType::Glow);
    add_particle_to_list(ParticleType::WaxOn);
    add_particle_to_list(ParticleType::WaxOff);
    add_particle_to_list(ParticleType::ElectricSpark);
    add_particle_to_list(ParticleType::Scrape);
}
