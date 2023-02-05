use valence_new::client::despawn_disconnected_clients;
use valence_new::client::event::default_event_handler;
use valence_new::prelude::*;
use valence_protocol::packets::s2c::particle::Particle;

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_system(manage_particles)
        .run();
}

#[derive(Resource)]
struct ParticleSpawner {
    particles: Vec<Particle>,
    index: usize,
}

impl ParticleSpawner {
    pub fn new() -> Self {
        Self {
            particles: create_particle_vec(),
            index: 0,
        }
    }

    pub fn next(&mut self) {
        self.index = (self.index + 1) % self.particles.len();
    }
}

fn setup(world: &mut World) {
    let mut instance = world
        .resource::<Server>()
        .new_instance(DimensionId::default());

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block_state([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    world.spawn(instance);

    let spawner = ParticleSpawner::new();
    world.insert_resource(spawner)
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    let instance = instances.get_single().unwrap();

    for mut client in &mut clients {
        client.set_position([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        client.set_instance(instance);
        client.set_game_mode(GameMode::Creative);
    }
}

fn manage_particles(
    mut spawner: ResMut<ParticleSpawner>,
    server: Res<Server>,
    mut clients: Query<&mut Client>,
) {
    if server.current_tick() % 20 == 0 {
        spawner.next();
    }

    if server.current_tick() % 5 != 0 {
        return;
    }

    let particle = &spawner.particles[spawner.index];
    let name = dbg_name(particle);

    let pos = [0.5, SPAWN_Y as f32 + 2.0, 2.5];
    let offset = [0.5, 0.5, 0.5];

    for mut client in clients.iter_mut() {
        client.set_action_bar(name.clone().bold());
        client.play_particle(particle, true, pos, offset, 0.1, 100);
    }
}

fn dbg_name(dbg: &impl std::fmt::Debug) -> String {
    let string = format!("{dbg:?}");

    string
        .split_once(|ch: char| !ch.is_ascii_alphabetic())
        .map(|(fst, _)| fst.to_owned())
        .unwrap_or(string)
}

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
            rgb: [1.0, 1.0, 0.0],
            scale: 2.0,
        },
        Particle::DustColorTransition {
            from_rgb: [1.0, 0.0, 0.0],
            scale: 2.0,
            to_rgb: [0.0, 1.0, 0.0],
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
            block_pos: [0, SPAWN_Y, 0].into(),
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
