#![allow(clippy::type_complexity)]

use std::f64::consts::TAU;

use glam::{DQuat, EulerRot};
use valence::client::{default_event_handler, despawn_disconnected_clients};
use valence::entity::player::PlayerEntityBundle;
use valence::prelude::*;

type SpherePartBundle = valence::entity::cow::CowEntityBundle;

const SPHERE_CENTER: DVec3 = DVec3::new(0.5, SPAWN_POS.y as f64 + 2.0, 0.5);
const SPHERE_AMOUNT: usize = 200;
const SPHERE_MIN_RADIUS: f64 = 6.0;
const SPHERE_MAX_RADIUS: f64 = 12.0;
const SPHERE_FREQ: f64 = 0.5;

const SPAWN_POS: BlockPos = BlockPos::new(0, 100, -16);

/// Marker component for entities that are part of the sphere.
#[derive(Component)]
struct SpherePart;

fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(default_event_handler.in_schedule(EventLoopSchedule))
        .add_systems(PlayerList::default_systems())
        .add_system(update_sphere)
        .add_system(despawn_disconnected_clients)
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Query<&DimensionType>,
    biomes: Query<&Biome>,
) {
    let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    instance.set_block(SPAWN_POS, BlockState::BEDROCK);

    let instance_id = commands.spawn(instance).id();

    commands.spawn_batch([0; SPHERE_AMOUNT].map(|_| {
        (
            SpherePartBundle {
                location: Location(instance_id),
                ..Default::default()
            },
            SpherePart,
        )
    }));
}

fn init_clients(
    mut clients: Query<(Entity, &UniqueId, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
    mut commands: Commands,
) {
    for (entity, uuid, mut game_mode) in &mut clients {
        *game_mode = GameMode::Creative;

        commands.entity(entity).insert(PlayerEntityBundle {
            location: Location(instances.single()),
            position: Position::new([
                SPAWN_POS.x as f64 + 0.5,
                SPAWN_POS.y as f64 + 1.0,
                SPAWN_POS.z as f64 + 0.5,
            ]),
            uuid: *uuid,
            ..Default::default()
        });
    }
}

fn update_sphere(
    server: Res<Server>,
    mut parts: Query<(&mut Position, &mut Look, &mut HeadYaw), With<SpherePart>>,
) {
    let time = server.current_tick() as f64 / server.tps() as f64;

    let rot_angles = DVec3::new(0.2, 0.4, 0.6) * SPHERE_FREQ * time * TAU % TAU;
    let rot = DQuat::from_euler(EulerRot::XYZ, rot_angles.x, rot_angles.y, rot_angles.z);

    let radius = lerp(
        SPHERE_MIN_RADIUS,
        SPHERE_MAX_RADIUS,
        ((time * SPHERE_FREQ * TAU).sin() + 1.0) / 2.0,
    );

    for ((mut pos, mut look, mut head_yaw), p) in
        parts.iter_mut().zip(fibonacci_spiral(SPHERE_AMOUNT))
    {
        debug_assert!(p.is_normalized());

        let dir = rot * p;

        pos.0 = SPHERE_CENTER + dir * radius;
        look.set_vec(dir.as_vec3());
        head_yaw.0 = look.yaw;
    }
}

/// Distributes N points on the surface of a unit sphere.
fn fibonacci_spiral(n: usize) -> impl Iterator<Item = DVec3> {
    let golden_ratio = (1.0 + 5_f64.sqrt()) / 2.0;

    (0..n).map(move |i| {
        // Map to unit square
        let x = i as f64 / golden_ratio % 1.0;
        let y = i as f64 / n as f64;

        // Map from unit square to unit sphere.
        let theta = x * TAU;
        let phi = (1.0 - 2.0 * y).acos();
        DVec3::new(theta.cos() * phi.sin(), theta.sin() * phi.sin(), phi.cos())
    })
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a * (1.0 - t) + b * t
}
