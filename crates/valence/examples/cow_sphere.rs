use std::f64::consts::TAU;

use glam::{DQuat, EulerRot};
use valence::client::despawn_disconnected_clients;
use valence::client::event::default_event_handler;
use valence::prelude::*;
use valence::util::to_yaw_and_pitch;

const SPHERE_CENTER: DVec3 = DVec3::new(0.5, SPAWN_POS.y as f64 + 2.0, 0.5);
const SPHERE_AMOUNT: usize = 200;
const SPHERE_KIND: EntityKind = EntityKind::Cow;
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

fn setup(mut commands: Commands, server: Res<Server>) {
    let mut instance = server.new_instance(DimensionId::default());

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    instance.set_block(SPAWN_POS, BlockState::BEDROCK);

    let instance_id = commands.spawn(instance).id();

    commands.spawn_batch(
        [0; SPHERE_AMOUNT].map(|_| (McEntity::new(SPHERE_KIND, instance_id), SpherePart)),
    );
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for mut client in &mut clients {
        client.set_position([
            SPAWN_POS.x as f64 + 0.5,
            SPAWN_POS.y as f64 + 1.0,
            SPAWN_POS.z as f64 + 0.5,
        ]);
        client.set_instance(instances.single());
        client.set_game_mode(GameMode::Creative);
    }
}

fn update_sphere(server: Res<Server>, mut parts: Query<&mut McEntity, With<SpherePart>>) {
    let time = server.current_tick() as f64 / server.tps() as f64;

    let rot_angles = DVec3::new(0.2, 0.4, 0.6) * SPHERE_FREQ * time * TAU % TAU;
    let rot = DQuat::from_euler(EulerRot::XYZ, rot_angles.x, rot_angles.y, rot_angles.z);

    let radius = lerp(
        SPHERE_MIN_RADIUS,
        SPHERE_MAX_RADIUS,
        ((time * SPHERE_FREQ * TAU).sin() + 1.0) / 2.0,
    );

    for (mut entity, p) in parts.iter_mut().zip(fibonacci_spiral(SPHERE_AMOUNT)) {
        debug_assert!(p.is_normalized());

        let dir = rot * p;
        let (yaw, pitch) = to_yaw_and_pitch(dir.as_vec3());

        entity.set_position(SPHERE_CENTER + dir * radius);
        entity.set_yaw(yaw);
        entity.set_head_yaw(yaw);
        entity.set_pitch(pitch);
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
